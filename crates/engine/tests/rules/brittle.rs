//! Brittle X (docs/classic-sets.md B3.3; R385, R438, R440, R441, R638): where the count lives, when it
//! starts, when it ticks (on the field only), what a crumble does, what a Vanilla does to it, what the
//! views show, and a crumble whose Death asks something pausing the settle after the tick (R113), with
//! the paused game surviving a JSON round trip and resuming identically.

use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;

use jackioh_engine::effects::reveal::reveal;
use jackioh_engine::effects::transform::vanilla;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{indestructible, plain, stacker};
use crate::rules::fixtures::harness::{events_of_type, in_hand, put, set_library, sink_for, slot};
use crate::rules::fixtures::instance_data::{asker, brittle_trap, brittle_unit, instance_game};

/// An `ActionInput` from a `json!` object literal (SURFACE §8).
fn input(body: Value) -> ActionInput {
    json_as(body)
}

fn by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

fn json_of(value: impl Serialize) -> Value {
    serde_json::to_value(value).expect("serialises")
}

fn instance_ids(events: impl Serialize) -> Vec<String> {
    json_of(events)
        .as_array()
        .map(|list| {
            list.iter()
                .map(|event| event["instanceId"].as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn by_id<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

fn by_id_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

fn live(state: &GameState, id: &str) -> CardInstance {
    by_id(state, id).clone()
}

fn count_of(state: &GameState, id: &str) -> Option<i32> {
    by_id(state, id).brittle.map(|brittle| brittle.count)
}

fn zone_of(state: &GameState, id: &str) -> ZoneName {
    by_id(state, id).zone.z()
}

/// A count, its `printed` absent unless `Some(true)`.
fn counter(count: i32, since: i32, printed: bool) -> BrittleCounter {
    BrittleCounter {
        count,
        since,
        printed: printed.then_some(true),
    }
}

fn with_live(state: &mut GameState, id: &str, change: impl FnOnce(&GameState, &mut CardInstance)) {
    let mut card = live(state, id);
    change(state, &mut card);
    *by_id_mut(state, id) = card;
}

/// A game at `turn`, `active`'s, with nothing else set up.
fn at(turn: i32, active: PlayerId) -> GameState {
    let mut state = instance_game("brittle", None);
    state.turn = turn;
    state.active = active;
    state.phase = Phase::Main;
    state
}

/// Run the tick for `player` at `turn`, as the start of their turn does, and settle after it.
fn tick_at(state: &mut GameState, turn: i32, player: PlayerId) -> Vec<GameEvent> {
    state.turn = turn;
    state.active = player;
    let mut sink = sink_for(state);
    brittle_tick(&mut sink, player);
    settle(&mut sink, SettleOptions::default());
    let cursor = sink.rng.cursor();
    sink.state.rng_cursor = cursor;
    sink.events.clone()
}

/// Give `card` a count of `count` as if on `turn`. `card` is the caller's copy: the live card when
/// it stands in `state` (both are written), or a card not placed yet.
fn given(state: &mut GameState, card: &mut CardInstance, count: i32, turn: i32) {
    let now = state.turn;
    state.turn = turn;
    if let Some(standing) = find_instance(state, &card.id) {
        *card = standing.clone();
    }
    give_brittle_count(state, card, count);
    if let Some(standing) = find_instance_mut(state, &card.id) {
        *standing = card.clone();
    }
    state.turn = now;
}

mod r385_b3_3_where_a_brittle_count_lives_and_when_it_starts {
    use super::*;

    #[test]
    fn r385_a_printed_brittle_starts_as_its_card_enters_the_field_and_not_before() {
        let mut state = at(4, P1);
        let held = in_hand(&mut state, &brittle_unit.id, P1, 1)
            .into_iter()
            .next()
            .expect("no card");
        assert_eq!(held.brittle, None);
        assert_eq!(active_brittle_count(&held), None);

        let unit = put(
            &mut state,
            &brittle_unit.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        assert_eq!(by_id(&state, &unit.id).brittle, Some(counter(2, 4, true)));
        let radiant = put(
            &mut state,
            &brittle_unit.id,
            slot(P1, Row::Units, 2),
            json_as(json!({ "radiant": true })),
        );
        assert_eq!(count_of(&state, &radiant.id), Some(4));
        // R687: a Field Trap set face-down has entered the field but is unrevealed, so it starts
        // no count — no Brittle while unrevealed (Classic+ #74).
        let trap = put(
            &mut state,
            &brittle_trap.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        assert_eq!(by_id(&state, &trap.id).brittle, None);
        let trap = live(&state, &trap.id);
        // The count starts when the card reveals.
        (reveal(Default::default()).apply)(&mut make_context(
            &mut sink_for(&mut state),
            Some(&trap),
            Default::default(),
        ));
        assert_eq!(by_id(&state, &trap.id).brittle, Some(counter(3, 4, true)));
    }

    #[test]
    fn r385_a_card_that_already_has_a_count_keeps_it_as_it_enters_the_field_so_a_count_is_kept_in_every_zone()
    {
        let mut state = at(6, P1);
        let mut card = new_instance(&mut state, &brittle_unit.id, P1, Zone::Hand { player: P1 });
        given(&mut state, &mut card, 5, 3);
        assert!(place_on_field(
            &mut state,
            &mut card,
            slot(P1, Row::Units, 1),
            Default::default()
        ));
        // R638: the count is kept, and its turn cycle starts on the field, at this arrival.
        assert_eq!(by_id(&state, &card.id).brittle, Some(counter(5, 6, false)));
    }

    #[test]
    fn r638_a_move_from_one_field_zone_to_another_is_no_arrival_the_counts_cycle_is_not_restarted() {
        let mut state = at(8, P1);
        let mut unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        given(&mut state, &mut unit, 3, 5);
        assert!(remove_from_field(&mut state, &unit, Default::default()));
        assert!(place_on_field(
            &mut state,
            &mut unit,
            slot(P1, Row::Units, 2),
            Default::default()
        ));
        assert_eq!(by_id(&state, &unit.id).brittle, Some(counter(3, 5, false)));
    }

    #[test]
    fn r385_a_count_ticks_first_at_its_controllers_start_of_turn_t_2_or_later_then_every_one_of_theirs() {
        assert_eq!(BRITTLE_FIRST_TICK_TURNS, 2);
        let mut state = at(5, P1);
        let mut mine = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        // Given on p1's turn 5: it lives through the rest of 5 and p2's turn 6, ticks at 7, crumbles at 9.
        given(&mut state, &mut mine, 2, 5);
        tick_at(&mut state, 5, P1);
        assert_eq!(count_of(&state, &mine.id), Some(2));
        tick_at(&mut state, 7, P1);
        assert_eq!(count_of(&state, &mine.id), Some(1));
        tick_at(&mut state, 9, P1);
        assert_eq!(zone_of(&state, &mine.id), ZoneName::Graveyard);
    }

    #[test]
    fn r385_a_count_given_on_the_other_players_turn_waits_a_whole_turn_cycle_of_its_own() {
        let mut state = at(6, P2);
        let mut mine = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        given(&mut state, &mut mine, 2, 6);
        // p1's turn 7 is only one player-turn later: no tick yet. Turn 9 is the first.
        tick_at(&mut state, 7, P1);
        assert_eq!(count_of(&state, &mine.id), Some(2));
        tick_at(&mut state, 9, P1);
        assert_eq!(count_of(&state, &mine.id), Some(1));
    }

    #[test]
    fn r385_the_other_players_start_of_turn_never_ticks_a_count() {
        let mut state = at(3, P1);
        let mut mine = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        given(&mut state, &mut mine, 1, 1);
        tick_at(&mut state, 4, P2);
        assert_eq!(count_of(&state, &mine.id), Some(1));
        assert_eq!(zone_of(&state, &mine.id), ZoneName::Field);
    }

    #[test]
    fn r638_a_count_in_a_hand_or_a_deck_holds_no_tick_no_crumble_and_no_event_however_long_it_waits() {
        let mut state = at(9, P1);
        let hand_card = in_hand(&mut state, &plain.id, P1, 1).into_iter().next();
        let deck_card = set_library(&mut state, P1, &[body(), plain.id.clone()])
            .into_iter()
            .next();
        let (Some(mut hand_card), Some(mut deck_card)) = (hand_card, deck_card) else {
            panic!("no card")
        };
        given(&mut state, &mut hand_card, 1, 3);
        given(&mut state, &mut deck_card, 2, 3);

        for turn in [9, 11, 13] {
            assert!(tick_at(&mut state, turn, P1).is_empty());
        }
        assert_eq!(by_id(&state, &hand_card.id).zone, Zone::Hand { player: P1 });
        assert_eq!(by_id(&state, &hand_card.id).brittle, Some(counter(1, 3, false)));
        assert_eq!(zone_of(&state, &deck_card.id), ZoneName::Library);
        assert_eq!(by_id(&state, &deck_card.id).brittle, Some(counter(2, 3, false)));
    }

    #[test]
    fn r638_a_count_held_in_a_hand_starts_its_cycle_as_the_card_enters_the_field_first_tick_at_t_2_of_the_arrival()
     {
        let mut state = at(5, P1);
        let mut card = new_instance(&mut state, &plain.id, P1, Zone::Hand { player: P1 });
        given(&mut state, &mut card, 2, 1);
        state.turn = 9;
        assert!(place_on_field(
            &mut state,
            &mut card,
            slot(P1, Row::Units, 1),
            Default::default()
        ));
        assert_eq!(by_id(&state, &card.id).brittle, Some(counter(2, 9, false)));

        // Held since turn 1, yet not due at 9: the cycle is the field's.
        tick_at(&mut state, 9, P1);
        assert_eq!(count_of(&state, &card.id), Some(2));
        tick_at(&mut state, 11, P1);
        assert_eq!(count_of(&state, &card.id), Some(1));
        tick_at(&mut state, 13, P1);
        assert_eq!(zone_of(&state, &card.id), ZoneName::Graveyard);
    }

    #[test]
    fn r638_a_card_that_leaves_the_field_keeps_the_count_it_has_and_it_holds_there_until_the_card_is_back() {
        let mut state = at(6, P1);
        let mut unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        given(&mut state, &mut unit, 3, 2);
        tick_at(&mut state, 6, P1);
        assert_eq!(count_of(&state, &unit.id), Some(2));

        let mut unit = live(&state, &unit.id);
        assert!(remove_from_field(&mut state, &unit, Default::default()));
        unit.zone = Zone::Hand { player: P1 };
        state.players.p1.hand.push(unit.clone());
        tick_at(&mut state, 8, P1);
        tick_at(&mut state, 10, P1);
        assert_eq!(count_of(&state, &unit.id), Some(2));
        assert_eq!(zone_of(&state, &unit.id), ZoneName::Hand);
    }
}

mod r385_b3_3_a_crumble {
    use super::*;

    #[test]
    fn r385_on_the_field_a_count_at_0_is_an_ordinary_destroy_the_state_check_collects_it_and_it_dies() {
        let mut state = at(7, P1);
        let mut unit = put(&mut state, &plain.id, slot(P1, Row::Units, 2), Default::default());
        given(&mut state, &mut unit, 1, 5);
        let events = tick_at(&mut state, 7, P1);
        assert_eq!(zone_of(&state, &unit.id), ZoneName::Graveyard);
        let kinds: Vec<GameEventType> = events
            .iter()
            .map(|event| event.event_type())
            .filter(|kind| {
                [
                    GameEventType::CounterChanged,
                    GameEventType::Crumbled,
                    GameEventType::Destroyed,
                ]
                .contains(kind)
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                GameEventType::CounterChanged,
                GameEventType::Crumbled,
                GameEventType::Destroyed
            ]
        );
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::CounterChanged))[0],
            json!({ "type": "counterChanged", "instanceId": unit.id, "counter": "brittle", "value": 0 })
        );
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::Crumbled))[0],
            json!({ "type": "crumbled", "instanceId": unit.id, "defId": plain.id, "owner": "p1", "zone": "field" })
        );
        // R441: the count that crumbled it is spent, and went with R78's reset.
        assert_eq!(by_id(&state, &unit.id).brittle, None);
    }

    #[test]
    fn r385_indestructible_ignores_the_crumble_the_count_stays_0_and_crumbles_it_again_at_each_tick() {
        let mut state = at(7, P1);
        let mut unit = put(
            &mut state,
            &indestructible.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        given(&mut state, &mut unit, 1, 5);
        let first = tick_at(&mut state, 7, P1);
        assert_eq!(zone_of(&state, &unit.id), ZoneName::Field);
        assert_eq!(count_of(&state, &unit.id), Some(0));
        assert_eq!(events_of_type(&first, GameEventType::Crumbled).len(), 1);
        assert_eq!(events_of_type(&first, GameEventType::Destroyed).len(), 0);

        let second = tick_at(&mut state, 9, P1);
        assert_eq!(zone_of(&state, &unit.id), ZoneName::Field);
        assert_eq!(count_of(&state, &unit.id), Some(0));
        assert_eq!(events_of_type(&second, GameEventType::Crumbled).len(), 1);
        // Nothing more to take off: no second counterChanged.
        assert_eq!(events_of_type(&second, GameEventType::CounterChanged).len(), 0);
    }

    #[test]
    fn r385_a_count_ticks_at_its_controllers_start_of_turn_on_the_field_a_stolen_card_ticks_on_the_thiefs() {
        let mut state = at(7, P1);
        let mut stolen = put(&mut state, &plain.id, slot(P1, Row::Units, 3), Default::default());
        by_id_mut(&mut state, &stolen.id).owner = P2;
        given(&mut state, &mut stolen, 3, 5);
        tick_at(&mut state, 8, P2);
        assert_eq!(count_of(&state, &stolen.id), Some(3));
        tick_at(&mut state, 9, P1);
        assert_eq!(count_of(&state, &stolen.id), Some(2));
    }

    #[test]
    fn r385_r13_a_card_dormant_under_a_stack_is_not_on_the_field_and_does_not_tick() {
        let mut state = at(7, P1);
        let mut under = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        given(&mut state, &mut under, 1, 5);
        let mut top = new_instance(&mut state, &stacker.id, P1, Zone::Hand { player: P1 });
        assert!(place_on_field(
            &mut state,
            &mut top,
            slot(P1, Row::Units, 1),
            json_as(json!({ "stack": true }))
        ));
        let events = tick_at(&mut state, 7, P1);
        assert_eq!(count_of(&state, &under.id), Some(1));
        assert_eq!(events_of_type(&events, GameEventType::Crumbled).len(), 0);
    }

    #[test]
    fn r385_r440_a_face_down_traps_count_ticks_silently_a_cue_would_tell_the_other_player_a_hidden_card_is_brittle()
     {
        let mut state = at(7, P1);
        let mut trap = put(
            &mut state,
            &brittle_trap.id,
            slot(P1, Row::Backrow, 2),
            Default::default(),
        );
        given(&mut state, &mut trap, 2, 5);
        let events = tick_at(&mut state, 7, P1);
        assert_eq!(count_of(&state, &trap.id), Some(1));
        assert!(events.is_empty());
    }
}

mod r385_r441_b3_3_rule_5_vanilla_and_the_count {
    use super::*;

    #[test]
    fn r385_a_vanilla_switches_a_printed_count_off_while_it_lasts_and_keeps_a_given_one() {
        let mut state = at(7, P1);
        let printed = put(
            &mut state,
            &brittle_unit.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let mut given_to = put(&mut state, &plain.id, slot(P1, Row::Units, 2), Default::default());
        given(&mut state, &mut given_to, 2, 5);
        by_id_mut(&mut state, &printed.id).brittle = Some(counter(2, 5, true));
        {
            let mut sink = sink_for(&mut state);
            let mut ctx = make_context(&mut sink, None, by(P1));
            (vanilla(json_as(json!({ "instanceId": printed.id }))).apply)(&mut ctx);
            (vanilla(json_as(json!({ "instanceId": given_to.id }))).apply)(&mut ctx);
        }

        assert_eq!(active_brittle_count(by_id(&state, &printed.id)), None);
        assert!(
            !unit_view(&state, by_id(&state, &printed.id))
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::Brittle)
        );
        assert_eq!(active_brittle_count(by_id(&state, &given_to.id)), Some(2));
        assert!(
            unit_view(&state, by_id(&state, &given_to.id))
                .keywords
                .contains(&Keyword::Brittle { n: 2 })
        );

        tick_at(&mut state, 7, P1);
        assert_eq!(count_of(&state, &printed.id), Some(2));
        assert_eq!(count_of(&state, &given_to.id), Some(1));
    }

    #[test]
    fn r385_the_count_in_force_is_the_units_brittle_keyword_a_given_one_on_a_card_that_prints_none_included()
    {
        let mut state = at(7, P1);
        let unit = put(
            &mut state,
            &brittle_unit.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        assert_eq!(
            unit_view(&state, by_id(&state, &unit.id)).keywords,
            vec![Keyword::Brittle { n: 2 }]
        );
        by_id_mut(&mut state, &unit.id).brittle = Some(counter(1, 7, true));
        assert_eq!(
            unit_view(&state, by_id(&state, &unit.id)).keywords,
            vec![Keyword::Brittle { n: 1 }]
        );
        let mut other = put(&mut state, &plain.id, slot(P1, Row::Units, 2), Default::default());
        given(&mut state, &mut other, 3, 7);
        assert_eq!(
            unit_view(&state, by_id(&state, &other.id)).keywords,
            vec![Keyword::Brittle { n: 3 }]
        );
    }

    #[test]
    fn r441_a_crumbled_card_that_comes_back_to_the_field_starts_its_printed_brittle_afresh() {
        let mut state = at(7, P1);
        let unit = put(
            &mut state,
            &brittle_unit.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        by_id_mut(&mut state, &unit.id).brittle = Some(counter(1, 5, true));
        tick_at(&mut state, 7, P1);
        assert_eq!(zone_of(&state, &unit.id), ZoneName::Graveyard);
        assert_eq!(by_id(&state, &unit.id).brittle, None);
        let mut unit = live(&state, &unit.id);
        state.players.p1.graveyard.retain(|card| card.id != unit.id);
        assert!(place_on_field(
            &mut state,
            &mut unit,
            slot(P1, Row::Units, 1),
            Default::default()
        ));
        assert_eq!(by_id(&state, &unit.id).brittle, Some(counter(2, 7, true)));
    }

    #[test]
    fn r441_gain_n_on_a_card_with_no_count_starts_one_now_at_n_more_than_it_prints_a_vanilla_keeps_it() {
        let mut state = at(4, P1);
        let card = in_hand(&mut state, &brittle_unit.id, P1, 1).into_iter().next();
        let bare = in_hand(&mut state, &plain.id, P1, 1).into_iter().next();
        let (Some(card), Some(bare)) = (card, bare) else {
            panic!("no card")
        };
        with_live(&mut state, &card.id, |state, card| {
            gain_brittle_count(state, card, 1)
        });
        with_live(&mut state, &bare.id, |state, card| {
            gain_brittle_count(state, card, 2)
        });
        assert_eq!(by_id(&state, &card.id).brittle, Some(counter(3, 4, false)));
        assert_eq!(by_id(&state, &bare.id).brittle, Some(counter(2, 4, false)));
        with_live(&mut state, &card.id, |state, card| {
            gain_brittle_count(state, card, 2)
        });
        assert_eq!(by_id(&state, &card.id).brittle, Some(counter(5, 4, false)));
    }
}

mod r385_b3_3_rule_6_who_sees_the_count {
    use super::*;

    #[test]
    fn r385_the_count_is_public_on_the_field_its_owners_alone_in_a_hand_and_a_face_down_traps_to_its_controller()
     {
        let mut state = at(3, P1);
        let mut unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        given(&mut state, &mut unit, 2, 3);
        let mut held = in_hand(&mut state, &plain.id, P1, 1)
            .into_iter()
            .next()
            .expect("no card");
        given(&mut state, &mut held, 2, 3);
        let trap = put(
            &mut state,
            &brittle_trap.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );

        let mine = view_for(&state, P1);
        let theirs = view_for(&state, P2);
        assert_eq!(mine.you.units[0].as_ref().and_then(|unit| unit.brittle), Some(2));
        assert_eq!(
            theirs.opponent.units[0].as_ref().and_then(|unit| unit.brittle),
            Some(2)
        );
        let HandView::Cards(hand) = &mine.you.hand else {
            panic!("own hand is a list")
        };
        assert_eq!(
            hand.iter()
                .find(|card| card.instance_id == held.id)
                .and_then(|card| card.brittle),
            Some(2)
        );
        assert_eq!(json_of(&theirs.opponent.hand), json!({ "count": 1 }));
        let own = match &mine.you.backrow[0] {
            Some(BackrowView::Public(public)) if !public.face_down => public.brittle,
            _ => None,
        };
        assert_eq!(own, count_of(&state, &trap.id));
        assert_eq!(
            json_of(&theirs.opponent.backrow[0]),
            json!({ "faceDown": true, "cost": 2 })
        );
        assert!(
            !serde_json::to_string(&theirs.opponent.backrow)
                .expect("serialises")
                .contains("brittle")
        );
    }
}

// A crumble whose Death asks (R113, §9.3)

static NONCE: AtomicU32 = AtomicU32::new(0);

fn act(state: &GameState, body: ActionInput, fixed: Option<&str>) -> GameState {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let nonce = fixed.map_or_else(|| format!("br{nonce}"), str::to_string);
    let result = reduce(state, &body.with_nonce(nonce));
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result.state
}

/// p1's main phase on turn 1, past the mulligans.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&instance_game(seed, None)).state;
    for player in [P1, P2] {
        let keep: Vec<String> = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        state = act(
            &state,
            input(json!({ "type": "mulligan", "keep": keep, "playerId": player })),
            None,
        );
    }
    state
}

mod r113_b3_3_a_crumble_that_pauses_the_settle_after_the_tick_9_3 {
    use super::*;

    #[test]
    fn r385_a_crumbled_units_death_prompt_parks_the_paused_game_survives_json_and_resumes_to_the_same_end() {
        let mut state = playing("brittle-pause");
        let unit = put(&mut state, &asker.id, slot(P1, Row::Units, 4), Default::default());
        let since = state.turn - BRITTLE_FIRST_TICK_TURNS;
        by_id_mut(&mut state, &unit.id).brittle = Some(counter(1, since, false));
        let other = put(&mut state, &plain.id, slot(P1, Row::Units, 5), Default::default());
        by_id_mut(&mut state, &other.id).brittle = Some(counter(1, since, false));

        let mut sink = sink_for(&mut state);
        brittle_tick(&mut sink, P1);
        settle(&mut sink, SettleOptions::default());
        let cursor = sink.rng.cursor();
        sink.state.rng_cursor = cursor;
        assert_eq!(
            instance_ids(events_of_type(sink.events, GameEventType::Crumbled)),
            vec![unit.id.clone(), other.id.clone()]
        );
        // Both crumbled together in one state check (R59), and the asker's Death is asking.
        assert_eq!(zone_of(&state, &unit.id), ZoneName::Graveyard);
        assert_eq!(zone_of(&state, &other.id), ZoneName::Graveyard);
        let pending = state.pending.clone();
        assert_eq!(pending.as_ref().map(|pending| pending.player_id), Some(P1));
        let pending = pending.expect("expected the Death to ask");

        let copy: GameState = serde_json::from_value(json_of(&state)).expect("a state survives JSON");
        assert_eq!(copy, clone_state(&state));
        let answer = json!({
            "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "mode", "option": "two" }],
            "playerId": "p1"
        });
        let live = act(&state, input(answer.clone()), Some("answer"));
        let replayed = act(&copy, input(answer), Some("answer"));
        assert_eq!(replayed, live);
        assert!(live.pending.is_none());
        assert!(live.work.is_empty());
        assert_eq!(live.players.p2.hero.health, state.players.p2.hero.health - 2);
    }

    #[test]
    fn r385_nothing_asks_nothing_parks_the_tick_and_its_check_finish_in_one_go() {
        let mut state = playing("brittle-quiet");
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 4), Default::default());
        let since = state.turn - BRITTLE_FIRST_TICK_TURNS;
        by_id_mut(&mut state, &unit.id).brittle = Some(counter(1, since, false));
        let mut sink = sink_for(&mut state);
        brittle_tick(&mut sink, P1);
        state_check(&mut sink);
        assert!(state.pending.is_none());
        assert!(state.work.is_empty());
        assert_eq!(zone_of(&state, &unit.id), ZoneName::Graveyard);
    }
}

/// A second vanilla fixture unit's id, for a deck that is not all one card.
fn body() -> String {
    "fx-2".to_string()
}
