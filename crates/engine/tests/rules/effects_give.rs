//! Cards between the players' piles (docs/classic-sets.md B5 E16, and E2's hand and deck half): a card
//! out of the other player's hand or deck becomes the taker's — owner and controller — so every pile
//! it reaches later is the taker's, the taker's hand cap burns it into the taker's graveyard, and a
//! draw from the other player's deck is a draw of the taker's own (Classic #9, #58, Classic+ #12.3).
//! R466 is the hidden-information half: `stolen` names the card only to whoever could read it where it
//! was taken or can read it now, and a card taken out of a library turns the rest of that library
//! unknown to its owner, whose list (R310) would otherwise name it by what it stopped listing.
//!
//! Port of `packages/engine/test/effects-give.test.ts`.

use jackioh_engine::effects::{draw_from_opponent, give_from_hand, take_from_library};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{events_of_type, in_hand, put, set_library, slot};
use crate::rules::fixtures::prompt_harness::{
    act, answer_keys, board, cast_now, expect_replays, hand_card, open_as, replayable, round_trip,
};
use crate::rules::fixtures::prompts::{
    cast_on_draw_marker, common_resources, fluffy_grip, glitch, grunt, income_tax, quickdraw_of,
};

/// TS `sinkFor(state)`: the events and rng of a sink over `state`, the rng starting at the state's
/// cursor as reduce does. The state is lent to it call by call (`on`).
struct Sink {
    events: Vec<GameEvent>,
    rng: Rng,
}

fn sink_for(state: &GameState) -> Sink {
    Sink {
        events: Vec::new(),
        rng: Rng::new(&state.seed, state.rng_cursor),
    }
}

impl Sink {
    fn on<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }
}

/// Apply one effect for `controller` outside any card, and record its events as an applied action.
fn run(state: &mut GameState, effect: Effect, controller: PlayerId) -> Vec<GameEvent> {
    let mut sink = sink_for(state);
    {
        let mut engine = sink.on(state);
        {
            let mut ctx = make_context(
                &mut engine,
                None,
                HookOptions {
                    controller: Some(controller),
                    ..Default::default()
                },
            );
            (effect.apply)(&mut ctx);
        }
        settle(&mut engine, Default::default());
    }
    state.rng_cursor = sink.rng.cursor();
    let nonce = format!("own-{}", state.applied.len());
    state.applied.push(AppliedAction {
        nonce,
        events: sink.events.clone(),
    });
    sink.events
}

/// The `stolen` events of the newest applied action, as `viewer` reads them.
fn stolen_seen_by(state: &GameState, viewer: PlayerId) -> Vec<Value> {
    view_for(state, viewer)
        .events
        .iter()
        .filter(|event| event.event_type() == GameEventType::Stolen)
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .collect()
}

fn ids_of(events: &[Value]) -> Vec<Value> {
    events
        .iter()
        .map(|event| event.get("instanceId").cloned().unwrap_or(Value::Null))
        .collect()
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).expect("the card is in the state")
}

fn live_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).expect("the card is in the state")
}

fn pluck<T: serde::Serialize>(events: &T, key: &str) -> Vec<Value> {
    match serde_json::to_value(events).expect("events serialise") {
        Value::Array(items) => items
            .into_iter()
            .map(|item| item.get(key).cloned().unwrap_or(Value::Null))
            .collect(),
        other => panic!("expected a list of events, got {other}"),
    }
}

fn unknown_in_own_library(state: &GameState, viewer: PlayerId) -> Option<i32> {
    view_for(state, viewer)
        .you
        .own_library
        .map(|library| library.unknown)
}

mod e16_e2_a_card_out_of_the_other_player_s_deck_classic_plus_12_3 {
    use super::*;

    #[test]
    fn r466_a_card_taken_out_of_the_other_player_s_deck_is_the_taker_s_reads_to_the_taker_alone_and_leaves_the_rest_of_that_deck_unknown_to_its_owner()
     {
        let mut state = board("fluffy");
        // A copy: the harness hands back the library array itself, which the steal splices.
        let library = set_library(
            &mut state,
            PlayerId::P2,
            &[glitch().id, plain.id.clone(), glitch().id],
        );
        set_library(&mut state, PlayerId::P1, std::slice::from_ref(&plain.id));
        assert_eq!(unknown_in_own_library(&state, PlayerId::P2), Some(0));
        let events = cast_now(&mut state, &fluffy_grip().id, PlayerId::P1, false).events;
        state.applied.push(AppliedAction {
            nonce: "fluffy".to_string(),
            events,
        });

        // The one Unit of their deck, now p1's in every sense, costing (0) in p1's hand.
        let taken = library[1].clone();
        assert_eq!(ids(&state.players[PlayerId::P1].hand), vec![taken.id.clone()]);
        assert_eq!(live(&state, &taken.id).owner, PlayerId::P1);
        assert_eq!(live(&state, &taken.id).controller, PlayerId::P1);
        assert_eq!(
            effective_cost(&state, live(&state, &taken.id), Default::default()),
            0
        );
        assert_eq!(live(&state, &taken.id).known_as, None);
        assert_eq!(
            ids(&state.players[PlayerId::P2].library),
            vec![library[0].id.clone(), library[2].id.clone()]
        );

        // `stolen`: the taker reads it; its old owner, who never saw their deck, reads only that a card left.
        assert_eq!(
            stolen_seen_by(&state, PlayerId::P1),
            vec![
                json!({ "type": "stolen", "instanceId": taken.id, "defId": plain.id, "from": "p2", "to": "p1", "zone": "library" })
            ]
        );
        assert_eq!(
            stolen_seen_by(&state, PlayerId::P2),
            vec![
                json!({ "type": "stolen", "instanceId": HIDDEN_ID, "defId": HIDDEN_ID, "from": "p2", "to": "p1", "zone": "library" })
            ]
        );
        let theirs = serde_json::to_string(&view_for(&state, PlayerId::P2).events).expect("events serialise");
        assert!(!theirs.contains(&taken.id));
        // R466: p2's list would have named the card by what it stopped listing; the rest is unknown now.
        assert_eq!(
            view_for(&state, PlayerId::P2).you.own_library,
            Some(LibraryView {
                cards: vec![],
                unknown: 2
            })
        );
        // The taker's own library is untouched.
        assert_eq!(unknown_in_own_library(&state, PlayerId::P1), Some(0));
        assert_eq!(
            view_for(&state, PlayerId::P1)
                .you
                .own_library
                .map(|library| library.cards.len()),
            Some(1)
        );

        // Played later, it is public, and the old event reads for both (R97: judged by where it is now).
        let mut played = live(&state, &taken.id).clone();
        played.zone = Zone::Graveyard { player: PlayerId::P1 };
        state.players[PlayerId::P1].hand = vec![];
        state.players[PlayerId::P1].graveyard.push(played);
        assert_eq!(
            ids_of(&stolen_seen_by(&state, PlayerId::P2)).first(),
            Some(&json!(taken.id))
        );
    }

    #[test]
    fn e16_radiant_makes_the_taken_card_radiant_with_no_unit_in_the_deck_nothing_is_taken() {
        let mut state = board("fluffy-radiant");
        set_library(&mut state, PlayerId::P2, std::slice::from_ref(&plain.id));
        cast_now(&mut state, &fluffy_grip().id, PlayerId::P1, true);
        assert_eq!(
            state.players[PlayerId::P1].hand.first().map(|card| card.radiant),
            Some(true)
        );
        assert_eq!(
            state.players[PlayerId::P1]
                .hand
                .first()
                .and_then(|card| card.cost_override),
            Some(0)
        );

        let mut empty = board("fluffy-none");
        set_library(&mut empty, PlayerId::P2, &[glitch().id, glitch().id]);
        let events = cast_now(&mut empty, &fluffy_grip().id, PlayerId::P1, false);
        assert_eq!(empty.players[PlayerId::P1].hand, Vec::<CardInstance>::new());
        assert!(events_of_type(&events, GameEventType::Stolen).is_empty());
        assert_eq!(unknown_in_own_library(&empty, PlayerId::P2), Some(0));
    }

    #[test]
    fn r218_e16_a_unit_token_card_never_leaves_a_library_for_a_hand_this_way() {
        let mut state = board("fluffy-token");
        set_library(&mut state, PlayerId::P2, &["fx-token-rush".to_string()]);
        cast_now(&mut state, &fluffy_grip().id, PlayerId::P1, false);
        assert_eq!(state.players[PlayerId::P1].hand, Vec::<CardInstance>::new());
        assert_eq!(state.players[PlayerId::P2].library.len(), 1);
    }

    #[test]
    fn e2_the_taker_s_hand_cap_burns_a_taken_card_into_the_taker_s_graveyard_and_no_price_rides_it_there() {
        let mut state = board("fluffy-burn");
        in_hand(&mut state, &plain.id, PlayerId::P1, HAND_CAP);
        let card = set_library(&mut state, PlayerId::P2, std::slice::from_ref(&plain.id))
            .into_iter()
            .next()
            .expect("the card");
        let events = cast_now(&mut state, &fluffy_grip().id, PlayerId::P1, false);
        assert_eq!(ids(&state.players[PlayerId::P1].graveyard), vec![card.id.clone()]);
        assert_eq!(live(&state, &card.id).owner, PlayerId::P1);
        assert_eq!(live(&state, &card.id).cost_override, None);
        assert_eq!(
            pluck(&events_of_type(&events, GameEventType::Burned), "owner"),
            vec![json!("p1")]
        );
    }

    #[test]
    fn e16_a_fluffy_grip_game_replays_from_its_log() {
        let qd = quickdraw_of(&fluffy_grip()).id;
        let dealt = replayable("fluffy-replay", std::slice::from_ref(&qd), &[]);
        let mut log = dealt.log;
        let decks = dealt.decks;
        let mut state = dealt.state;
        let played = hand_card(&state, PlayerId::P1, &qd).id.clone();
        state = act(
            &state,
            json!({ "type": "play", "playerId": "p1", "instanceId": played }),
            Some(&mut log),
        );
        assert!(
            state.players[PlayerId::P1]
                .hand
                .iter()
                .any(|card| card.owner == PlayerId::P1 && card.cost_override == Some(0))
        );
        expect_replays("fluffy-replay", &decks, &log, &state);
    }
}

mod e16_cards_handed_from_one_hand_to_the_other_classic_9 {
    use super::*;

    fn taxed(seed: &str, radiant: bool) -> GameState {
        let mut state = board(seed);
        let trap = put(
            &mut state,
            &income_tax().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        live_mut(&mut state, &trap.id).radiant = radiant;
        in_hand(&mut state, &plain.id, PlayerId::P2, 2);
        in_hand(&mut state, &grunt().id, PlayerId::P2, 1);
        set_library(&mut state, PlayerId::P2, &[glitch().id, plain.id.clone()]);
        state.active = PlayerId::P2;
        let mut sink = sink_for(&state);
        {
            let mut engine = sink.on(&mut state);
            jackioh_engine::draw::draw(&mut engine, PlayerId::P2, 1);
            settle(&mut engine, Default::default());
        }
        state.rng_cursor = sink.rng.cursor();
        state.applied.push(AppliedAction {
            nonce: format!("{seed}-draw"),
            events: sink.events,
        });
        state
    }

    #[test]
    fn r466_the_other_player_keeps_one_card_of_their_choice_and_gives_the_rest_a_victim_who_lost_a_card_from_their_own_hand_reads_it()
     {
        let mut state = taxed("tax", false);
        let pending = open_as(&state, PromptKind::Hand, PlayerId::P2);
        let hand = state.players[PlayerId::P2].hand.clone();
        assert_eq!(
            pending
                .options
                .iter()
                .map(|option| option.key.clone())
                .collect::<Vec<_>>(),
            hand.iter()
                .map(|card| format!("instance:{}", card.id))
                .collect::<Vec<_>>()
        );
        assert_eq!(pending.min, 1);
        assert_eq!(pending.max, 1);
        // The trap's controller sees that the other player is choosing, and nothing of their hand.
        assert_eq!(
            view_for(&state, PlayerId::P1).pending,
            Some(PendingView::Elsewhere(PendingElsewhereView {
                for_you: false,
                pending_for: PlayerId::P2
            }))
        );
        let kept = hand[3].clone();
        let key = format!("instance:{}", kept.id);

        let mut copy = round_trip(&state);
        let answered = answer_keys(&mut state, &[key.as_str()]);
        answer_keys(&mut copy, &[key.as_str()]);
        assert_eq!(hash_state(&copy), hash_state(&state));
        state.applied.push(AppliedAction {
            nonce: "tax-answer".to_string(),
            events: answered.events,
        });

        assert_eq!(ids(&state.players[PlayerId::P2].hand), vec![kept.id.clone()]);
        let given: Vec<CardInstance> = hand.iter().filter(|card| card.id != kept.id).cloned().collect();
        assert_eq!(ids(&state.players[PlayerId::P1].hand), ids(&given));
        for card in &given {
            assert_eq!(live(&state, &card.id).owner, PlayerId::P1);
            assert_eq!(live(&state, &card.id).controller, PlayerId::P1);
        }
        // A card out of a hand reads to the hand's holder, who held it, and to its new holder (R466).
        for viewer in [PlayerId::P1, PlayerId::P2] {
            assert_eq!(
                ids_of(&stolen_seen_by(&state, viewer)),
                given.iter().map(|card| json!(card.id)).collect::<Vec<_>>()
            );
        }
        // The spent trap reached its owner's graveyard.
        assert_eq!(
            state.players[PlayerId::P1]
                .graveyard
                .iter()
                .map(|card| card.def_id.clone())
                .collect::<Vec<_>>(),
            vec![income_tax().id]
        );
    }

    #[test]
    fn e16_radiant_the_cards_handed_over_cost_1_less_in_the_taker_s_hand() {
        let mut state = taxed("tax-radiant", true);
        let kept = state.players[PlayerId::P2].hand[0].clone();
        answer_keys(&mut state, &[format!("instance:{}", kept.id).as_str()]);
        assert_eq!(
            state.players[PlayerId::P1]
                .hand
                .iter()
                .map(|card| card.cost_mod)
                .collect::<Vec<_>>(),
            vec![-1, -1, -1]
        );
        assert_eq!(live(&state, &kept.id).cost_mod, 0);
    }

    #[test]
    fn e16_cards_may_be_given_the_other_way_and_at_random() {
        let mut state = board("give-away");
        let mine = in_hand(&mut state, &plain.id, PlayerId::P1, 3);
        run(
            &mut state,
            give_from_hand(json_as(json!({ "from": "self", "cards": "random", "count": 2 }))),
            PlayerId::P1,
        );
        assert_eq!(state.players[PlayerId::P1].hand.len(), 1);
        assert_eq!(state.players[PlayerId::P2].hand.len(), 2);
        for card in &state.players[PlayerId::P2].hand {
            assert!(ids(&mine).contains(&card.id));
            assert_eq!(card.owner, PlayerId::P2);
        }
        // p1 held them: p1 reads the `stolen` events; p2 holds them now and reads them too.
        assert!(
            ids_of(&stolen_seen_by(&state, PlayerId::P1))
                .iter()
                .all(|id| *id != json!(HIDDEN_ID))
        );
    }
}

mod e16_a_draw_from_the_other_player_s_deck_classic_58 {
    use super::*;

    #[test]
    fn e16_it_is_a_draw_of_the_drawer_s_own_from_the_bottom_of_their_deck() {
        let mut state = board("resources");
        let library = set_library(
            &mut state,
            PlayerId::P2,
            &[glitch().id, grunt().id, plain.id.clone()],
        );
        let drawn_before = state.counters.drawn;
        let events = run(&mut state, draw_from_opponent(Default::default()), PlayerId::P1);
        let bottom = library[2].clone();
        assert_eq!(ids(&state.players[PlayerId::P1].hand), vec![bottom.id.clone()]);
        assert_eq!(live(&state, &bottom.id).owner, PlayerId::P1);
        assert_eq!(
            ids(&state.players[PlayerId::P2].library),
            vec![library[0].id.clone(), library[1].id.clone()]
        );
        assert_eq!(state.counters.drawn, drawn_before + 1);
        assert_eq!(
            serde_json::to_value(events_of_type(&events, GameEventType::Drawn)).expect("events serialise"),
            json!([{ "type": "drawn", "player": "p1", "instanceId": bottom.id, "defId": plain.id, "turnDraw": 1 }])
        );
        // The drawer reads it; the deck's owner reads that p1 drew, never what (R466).
        let theirs = view_for(&state, PlayerId::P2).events;
        assert_eq!(
            pluck(&events_of_type(&theirs, GameEventType::Drawn), "instanceId").first(),
            Some(&json!(HIDDEN_ID))
        );
        assert_eq!(
            ids_of(&stolen_seen_by(&state, PlayerId::P2)).first(),
            Some(&json!(HIDDEN_ID))
        );
        assert_eq!(
            view_for(&state, PlayerId::P2).you.own_library,
            Some(LibraryView {
                cards: vec![],
                unknown: 2
            })
        );
    }

    #[test]
    fn e16_an_empty_enemy_deck_gives_nothing_and_fatigue_to_nobody() {
        let mut state = board("resources-empty");
        state.players[PlayerId::P2].library = vec![];
        let events = run(&mut state, draw_from_opponent(Default::default()), PlayerId::P1);
        assert_eq!(events, Vec::<GameEvent>::new());
        assert_eq!(state.players[PlayerId::P1].fatigue_count, 0);
        assert_eq!(state.players[PlayerId::P2].fatigue_count, 0);
        assert_eq!(state.players[PlayerId::P1].hero.health, HERO_HEALTH);
        let mut sink = sink_for(&state);
        assert!(
            draw_from_library_of(
                &mut sink.on(&mut state),
                PlayerId::P1,
                PlayerId::P2,
                Default::default()
            )
            .is_none()
        );
    }

    #[test]
    fn r58_e16_a_cast_on_draw_card_drawn_from_their_deck_is_cast_for_the_drawer() {
        let mut state = board("resources-cast");
        set_library(
            &mut state,
            PlayerId::P2,
            &[plain.id.clone(), cast_on_draw_marker().id],
        );
        run(&mut state, draw_from_opponent(Default::default()), PlayerId::P1);
        // The marker's Cry hits its caster's enemy: p2, whose deck it came from.
        assert_eq!(state.players[PlayerId::P2].hero.health, HERO_HEALTH - 2);
        assert_eq!(state.players[PlayerId::P1].hero.health, HERO_HEALTH);
        assert_eq!(
            state.players[PlayerId::P1]
                .graveyard
                .iter()
                .map(|card| card.def_id.clone())
                .collect::<Vec<_>>(),
            vec![cast_on_draw_marker().id]
        );
    }

    #[test]
    fn r4_e16_the_drawer_s_hand_cap_burns_a_card_drawn_from_their_deck_into_the_drawer_s_graveyard() {
        let mut state = board("resources-burn");
        in_hand(&mut state, &plain.id, PlayerId::P1, HAND_CAP);
        let card = set_library(&mut state, PlayerId::P2, &[grunt().id])
            .into_iter()
            .next()
            .expect("the card");
        run(
            &mut state,
            draw_from_opponent(json_as(json!({ "end": "top" }))),
            PlayerId::P1,
        );
        assert_eq!(ids(&state.players[PlayerId::P1].graveyard), vec![card.id.clone()]);
        assert_eq!(state.players[PlayerId::P2].graveyard, Vec::<CardInstance>::new());
    }

    #[test]
    fn e16_common_resources_draws_at_its_controller_s_start_of_turn() {
        let mut state = board("resources-turn");
        put(
            &mut state,
            &common_resources().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        set_library(&mut state, PlayerId::P1, &[plain.id.clone(), plain.id.clone()]);
        let theirs = set_library(
            &mut state,
            PlayerId::P2,
            &[plain.id.clone(), grunt().id, glitch().id],
        );
        state.active = PlayerId::P2;
        state = act(&state, json!({ "type": "endTurn", "playerId": "p2" }), None);
        assert_eq!(state.active, PlayerId::P1);
        // Their bottom card, and p1's own draw of the turn.
        assert!(ids(&state.players[PlayerId::P1].hand).contains(&theirs[2].id));
        assert_eq!(state.players[PlayerId::P1].hand.len(), 2);
        assert_eq!(state.players[PlayerId::P2].library.len(), 2);
    }
}

mod r466_a_card_taken_off_the_field {
    use super::*;

    #[test]
    fn r466_a_face_up_card_reads_to_both_a_face_down_one_to_whoever_controlled_it_neither_once_hidden_from_them_now()
     {
        let mut state = board("field-steal");
        let face_up = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let own_trap = put(
            &mut state,
            &income_tax().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        // p2's trap, taken over by p1 earlier (R33: the controller reads a face-down trap, not its owner).
        let taken_trap = put(
            &mut state,
            &income_tax().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        live_mut(&mut state, &taken_trap.id).owner = PlayerId::P2;
        let mut sink = sink_for(&state);
        for id in [&face_up.id, &own_trap.id, &taken_trap.id] {
            let card = live(&state, id).clone();
            take_into_hand(&mut sink.on(&mut state), &card, PlayerId::P1);
        }
        state.applied.push(AppliedAction {
            nonce: "field-steal".to_string(),
            events: sink.events.clone(),
        });
        assert_eq!(
            pluck(
                &events_of_type(&sink.events, GameEventType::Stolen),
                "readableFrom"
            ),
            vec![json!(["p1", "p2"]), json!(["p2"]), json!(["p1"])]
        );
        // p1 holds all three now, so p1 reads all three.
        assert_eq!(
            ids_of(&stolen_seen_by(&state, PlayerId::P1)),
            vec![json!(face_up.id), json!(own_trap.id), json!(taken_trap.id)]
        );
        // p2 saw the face-up unit and controlled its own face-down trap: it knows what it lost. The trap p1
        // controlled face-down it never read, so it learns only that a card left.
        assert_eq!(
            ids_of(&stolen_seen_by(&state, PlayerId::P2)),
            vec![json!(face_up.id), json!(own_trap.id), json!(HIDDEN_ID)]
        );
    }
}

mod e2_e16_the_change_of_owner_itself {
    use super::*;

    #[test]
    fn e2_a_card_that_has_ceased_to_exist_is_not_taken() {
        let mut state = board("gone");
        let card = new_instance(
            &mut state,
            &plain.id,
            PlayerId::P2,
            Zone::Gone { player: PlayerId::P2 },
        );
        let mut sink = sink_for(&state);
        assert!(take_into_hand(&mut sink.on(&mut state), &card, PlayerId::P1).is_none());
        assert!(!change_owner(&mut sink.on(&mut state), &card, PlayerId::P1));
        // TS read the object it handed in; nothing in the state holds a card that is gone.
        assert_eq!(card.owner, PlayerId::P2);
        assert!(find_instance(&state, &card.id).is_none());
        assert_eq!(sink.events, Vec::<GameEvent>::new());
    }

    #[test]
    fn r466_a_stolen_card_reads_to_whoever_could_read_it_where_it_was_taken_or_can_read_it_now_the_stack_and_public_piles_to_both()
     {
        let mut state = board("public");
        let resolving = new_instance(
            &mut state,
            &glitch().id,
            PlayerId::P2,
            Zone::Resolving { player: PlayerId::P2 },
        );
        state.players[PlayerId::P2].resolving.push(resolving.clone());
        let dead = new_instance(
            &mut state,
            &plain.id,
            PlayerId::P2,
            Zone::Graveyard { player: PlayerId::P2 },
        );
        state.players[PlayerId::P2].graveyard.push(dead.clone());
        let mut sink = sink_for(&state);
        take_into_hand(&mut sink.on(&mut state), &resolving, PlayerId::P1);
        take_into_hand(&mut sink.on(&mut state), &dead, PlayerId::P1);
        state.applied.push(AppliedAction {
            nonce: "public".to_string(),
            events: sink.events.clone(),
        });
        assert_eq!(
            ids(&state.players[PlayerId::P1].hand),
            vec![resolving.id.clone(), dead.id.clone()]
        );
        for viewer in [PlayerId::P1, PlayerId::P2] {
            assert_eq!(
                ids_of(&stolen_seen_by(&state, viewer)),
                vec![json!(resolving.id), json!(dead.id)]
            );
        }
        // The event carries who could read the card where it was taken, and no view forwards it.
        for viewer in [PlayerId::P1, PlayerId::P2] {
            let events = serde_json::to_string(&view_for(&state, viewer).events).expect("events serialise");
            assert!(!events.contains("readableFrom"));
        }
        // And the other way round: p2 takes the top of p1's deck, and the rest of it turns unknown to p1.
        set_library(&mut state, PlayerId::P1, &[plain.id.clone(), grunt().id]);
        let own = state.players[PlayerId::P1].library[0].clone();
        run(
            &mut state,
            take_from_library(json_as(json!({ "from": "enemy", "pick": "top" }))),
            PlayerId::P2,
        );
        assert_eq!(live(&state, &own.id).owner, PlayerId::P2);
        assert_eq!(unknown_in_own_library(&state, PlayerId::P1), Some(1));
    }
}

mod e16_and_e3_a_draw_from_the_other_player_s_deck_is_the_drawer_s_draw_for_their_draw_limit_classic_58_49 {
    use super::*;

    #[test]
    fn e3_a_draw_past_the_drawer_s_limit_takes_no_card_from_the_other_deck_draw_limited_and_nothing_moves() {
        let mut state = board("resources-limited");
        let library = set_library(&mut state, PlayerId::P2, &[grunt().id, plain.id.clone()]);
        // A permanent of p2's that lets p1 draw 1 card each turn (B5 E3), and p1 has drawn once this turn.
        let limiter_id = "give-limiter";
        let mut catalog = registered_catalog().clone();
        catalog.insert(
            limiter_id.to_string(),
            CardDef {
                id: limiter_id.to_string(),
                index: limiter_id.to_string(),
                ..plain.clone()
            },
        );
        register_catalog(catalog);
        let limit = Script {
            draw_limit: Some(read_hook(|_args| {
                vec![DrawLimit {
                    player: DrawLimitPlayer::Enemy,
                    count: 1,
                }]
            })),
            ..Script::default()
        };
        let mut registry = registered_scripts().clone();
        registry.insert(
            limiter_id.to_string(),
            CardScripts {
                base: limit.clone(),
                radiant: limit,
            },
        );
        register_scripts(registry);
        put(
            &mut state,
            limiter_id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        state.players[PlayerId::P1].draws = Some(DrawCount {
            turn: state.turn,
            count: 1,
        });

        let events = run(&mut state, draw_from_opponent(Default::default()), PlayerId::P1);
        assert_eq!(
            serde_json::to_value(events_of_type(&events, GameEventType::DrawLimited))
                .expect("events serialise"),
            json!([{ "type": "drawLimited", "player": "p1" }])
        );
        assert_eq!(ids(&state.players[PlayerId::P2].library), ids(&library));
        assert_eq!(live(&state, &library[1].id).owner, PlayerId::P2);
        assert_eq!(state.players[PlayerId::P1].hand.len(), 0);
    }
}
