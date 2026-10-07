//! E29 board snapshots (docs/classic-sets.md B5 E29; SPEC §2.2, §10.1; R419, R562, R563, R566, R227): the
//! record at each turn's start, the restore's three steps (`restoreBoard`), the R227 rename that keeps the
//! history naming a card by its current id, R563's Reborn hold, the views, and a Rollback played through
//! `reduce` that folds back from its log. Fixtures: `fixtures/boardHistory.ts`, and the field fixtures'
//! Frostspatula and Ivory Tower shapes (`fixtures/field.ts`). The card itself: C+ #35's test file.
//!
//! Port of `packages/engine/test/boardHistory.test.ts`.

use jackioh_engine::subsystems::board_history::{record_board_snapshot, restore_board, snapshot_for};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::board_history::{phoenix, register_board_history_fixtures, rewind};
use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::field::{act, banner, playing, spatula, tower, watcher};
use crate::rules::fixtures::harness::{events_of_type, put, setup_catalog, slot};

/// p1's main phase of turn 1 with the field fixtures and this file's registered, the history emptied.
fn board(seed: &str) -> GameState {
    let mut state = playing(seed);
    register_board_history_fixtures();
    state.board_history = None;
    state
}

const PLAIN_UNIT: &str = "fx-1";

fn input(body: Value) -> ActionInput {
    json_as(body)
}

fn json_of<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).unwrap()
}

/// vitest's `toMatchObject`: every key the expected object names matches, recursively; an array
/// matches element for element and in length.
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

/// The live card (TS held the object itself; Rust looks it up by id).
fn card<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

fn card_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

/// The TS `restoreBoard(sinkFor(state, events), …)`: a sink whose rng starts at the state's cursor.
fn restore(
    state: &mut GameState,
    events: &mut Vec<GameEvent>,
    by: PlayerId,
    turns_ago: i32,
    only: &[PlayerId],
) -> Option<i32> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, events, &mut rng);
    restore_board(&mut sink, by, turns_ago, only)
}

fn ids_of<C: std::borrow::Borrow<CardInstance>>(cards: &[C]) -> Vec<String> {
    cards.iter().map(|card| card.borrow().id.clone()).collect()
}

fn instance_ids<E: serde::Serialize>(events: &[E]) -> Vec<Value> {
    events.iter().map(|event| json_of(event)["instanceId"].clone()).collect()
}

fn history_turns(state: &GameState) -> Option<Vec<i32>> {
    state
        .board_history
        .as_ref()
        .map(|history| history.iter().map(|snapshot| snapshot.turn).collect())
}

fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(json_of(state)).unwrap()
}

mod e29_the_record_r419_r62 {
    use super::*;

    #[test]
    fn r419_a_turns_start_records_the_field_before_anything_else_an_animated_on_your_turn_card_is_still_in_its_backrow_zone_and_keeps_board_history_depth()
     {
        let mut state = playing("bh-record");
        assert_eq!(history_turns(&state), Some(vec![1]));
        put(&mut state, &spatula().id, slot(P1, Row::Backrow, 2));
        let unit = put(&mut state, PLAIN_UNIT, slot(P1, Row::Units, 1));
        card_mut(&mut state, &unit.id).damage = 1;
        state = act(&state, input(json!({ "type": "endTurn", "playerId": "p1" })));
        state = act(&state, input(json!({ "type": "endTurn", "playerId": "p2" })));

        let snapshot = state
            .board_history
            .as_ref()
            .and_then(|history| history.last())
            .cloned()
            .expect("a snapshot");
        assert_eq!(snapshot.turn, 3);
        assert_eq!(
            snapshot.sides.p1.backrow[1].as_ref().map(|card| card.def_id.clone()),
            Some(spatula().id)
        );
        assert_eq!(
            card_at(&state, slot(P1, Row::Units, 2)).map(|card| card.def_id.clone()),
            Some(spatula().id)
        );
        assert!(matches_object(
            &json_of(snapshot.sides.p1.units[0].as_ref().and_then(|pile| pile.first())),
            &json!({ "id": unit.id, "damage": 1 })
        ));
        assert_eq!(snapshot.sides.p2.locks, state.players.p2.locks);

        for _ in 0..4 {
            let active = state.active;
            state = act(&state, input(json!({ "type": "endTurn", "playerId": active })));
        }
        assert_eq!(BOARD_HISTORY_DEPTH, 4);
        assert_eq!(history_turns(&state), Some(vec![4, 5, 6, 7]));
        // §9.3: plain data.
        let history = state.board_history.clone();
        assert_eq!(
            serde_json::from_value::<Option<Vec<BoardSnapshot>>>(json_of(&history)).unwrap(),
            history
        );
    }

    #[test]
    fn r562_the_lookup_the_snapshot_n_turns_back_else_the_oldest_held_none_before_the_first_turn() {
        let mut state = board("bh-lookup");
        assert!(snapshot_for(&state, 1).is_none());
        state.turn = 5;
        record_board_snapshot(&mut state);
        state.turn = 6;
        record_board_snapshot(&mut state);
        assert_eq!(snapshot_for(&state, 1).map(|snapshot| snapshot.turn), Some(5));
        assert_eq!(snapshot_for(&state, 3).map(|snapshot| snapshot.turn), Some(5));
    }
}

mod e29_the_restore_r419 {
    use super::*;

    #[test]
    fn r419_puts_back_cards_from_a_library_a_hand_a_graveyard_and_exile_rebuilds_a_stack_pile_a_backrow_pile_and_a_carried_unit_and_restores_the_locks()
     {
        let mut state = board("bh-restore");
        let from_library = put(&mut state, PLAIN_UNIT, slot(P1, Row::Units, 1));
        let from_hand = put(&mut state, "fx-2", slot(P1, Row::Units, 2));
        let from_graveyard = put(&mut state, "fx-3", slot(P2, Row::Units, 1));
        let from_exile = put(&mut state, "fx-4", slot(P2, Row::Units, 2));
        let top = new_instance(&mut state, "fx-5", P1, Zone::Hand { player: P1 });
        place_on_field(
            &mut state,
            top.clone(),
            slot(P1, Row::Units, 1),
            PlaceOnFieldOptions { stack: Some(true) },
        );
        let carrier = put(&mut state, &tower().id, slot(P1, Row::Backrow, 1));
        let carried = new_instance(&mut state, "fx-6", P1, Zone::Hand { player: P1 });
        place_on_field(
            &mut state,
            carried.clone(),
            slot(P1, Row::Backrow, 1),
            PlaceOnFieldOptions::default(),
        );
        let trap = put(&mut state, &watcher().id, slot(P2, Row::Backrow, 3));
        // A backrow pile (B5 E21): a face-up Field Spell over a face-down trap.
        let under = put(&mut state, &watcher().id, slot(P1, Row::Backrow, 2));
        let over = new_instance(&mut state, &banner().id, P1, Zone::Hand { player: P1 });
        place_on_field(
            &mut state,
            over.clone(),
            slot(P1, Row::Backrow, 2),
            PlaceOnFieldOptions { stack: Some(true) },
        );
        lock_zone(&mut state, slot(P2, Row::Units, 5));
        record_board_snapshot(&mut state);

        move_to_zone(&mut state, &from_library.id, OffFieldZone::Library, MoveToZoneOptions::default());
        move_to_zone(&mut state, &from_hand.id, OffFieldZone::Hand, MoveToZoneOptions::default());
        move_to_zone(&mut state, &from_graveyard.id, OffFieldZone::Graveyard, MoveToZoneOptions::default());
        move_to_zone(&mut state, &from_exile.id, OffFieldZone::Exile, MoveToZoneOptions::default());
        move_to_zone(&mut state, &carried.id, OffFieldZone::Hand, MoveToZoneOptions::default());
        move_to_zone(&mut state, &over.id, OffFieldZone::Hand, MoveToZoneOptions::default());
        assert_eq!(
            card_at(&state, slot(P1, Row::Backrow, 2)).map(|card| card.id.clone()),
            Some(under.id.clone())
        );
        let newcomer = put(&mut state, "fx-7", slot(P2, Row::Units, 4));
        lock_zone(&mut state, slot(P1, Row::Units, 3));
        state.players.p2.locks.units[4] = false;

        let mut events: Vec<GameEvent> = Vec::new();
        assert_eq!(restore(&mut state, &mut events, P1, 1, &[P1, P2]), Some(0));
        assert_eq!(
            ids_of(&zone_contents(&state, slot(P1, Row::Units, 1))),
            vec![top.id.clone(), from_library.id.clone()]
        );
        assert_eq!(
            card_at(&state, slot(P1, Row::Units, 2)).map(|card| card.id.clone()),
            Some(from_hand.id.clone())
        );
        assert_eq!(
            card_at(&state, slot(P2, Row::Units, 1)).map(|card| card.id.clone()),
            Some(from_graveyard.id.clone())
        );
        assert_eq!(
            card_at(&state, slot(P2, Row::Units, 2)).map(|card| card.id.clone()),
            Some(from_exile.id.clone())
        );
        assert_eq!(
            ids_of(&zone_contents(&state, slot(P1, Row::Backrow, 1))),
            vec![carried.id.clone(), carrier.id.clone()]
        );
        assert_eq!(
            card_at(&state, slot(P2, Row::Backrow, 3)).map(|card| card.id.clone()),
            Some(trap.id.clone())
        );
        // The pile is rebuilt; the trap beneath never left the field face-down, so it keeps its id (R227).
        assert_eq!(
            ids_of(&zone_contents(&state, slot(P1, Row::Backrow, 2))),
            vec![over.id.clone(), under.id.clone()]
        );
        assert_eq!(card(&state, &newcomer.id).zone.z(), ZoneName::Hand);
        assert!(!is_locked(&state, slot(P1, Row::Units, 3)));
        assert!(is_locked(&state, slot(P2, Row::Units, 5)));
        let side = &state.players.p1;
        for pile in [&side.hand, &side.library, &side.graveyard, &side.exile] {
            assert!(!ids_of(pile).contains(&from_library.id));
        }
        let types: Vec<GameEventType> = events.iter().map(|event| event.event_type()).collect();
        assert_eq!(types[0], GameEventType::RolledBack);
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::Locked)),
            json!([{ "type": "locked", "player": "p2", "row": "units", "lane": 5 }])
        );
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::Unlocked)),
            json!([{ "type": "unlocked", "player": "p1", "row": "units", "lane": 3 }])
        );
        // The face-down trap never moved: no fresh id and no event names it.
        assert!(!instance_ids(&events_of_type(&events, GameEventType::ControlChanged)).contains(&json!(trap.id)));
    }

    #[test]
    fn r227_a_card_that_takes_a_fresh_id_is_renamed_in_the_history_so_a_re_set_trap_goes_back_as_itself_not_as_a_second_copy()
     {
        let mut state = board("bh-rename");
        let trap = put(&mut state, &watcher().id, slot(P1, Row::Backrow, 2));
        let first = trap.id.clone();
        record_board_snapshot(&mut state);
        move_to_zone(&mut state, &trap.id, OffFieldZone::Hand, MoveToZoneOptions::default());
        // TS renamed and placed the very object the hand held; Rust takes that card out of the hand
        // first, so it is "a card that is in no pile" as `freshFaceDownId` asks, and is placed once.
        let at = state
            .players
            .p1
            .hand
            .iter()
            .position(|held| held.id == trap.id)
            .expect("the trap went back to the hand");
        let mut moved = state.players.p1.hand.remove(at);
        fresh_face_down_id(&mut state, &mut moved);
        let trap_id = moved.id.clone();
        place_on_field(&mut state, moved, slot(P1, Row::Backrow, 4), PlaceOnFieldOptions::default());
        assert_eq!(
            state.board_history.as_ref().and_then(|history| history.first()).and_then(|snapshot| snapshot
                .sides
                .p1
                .backrow[1]
                .as_ref()
                .map(|card| card.id.clone())),
            Some(trap_id.clone())
        );
        assert_ne!(trap_id, first);

        restore(&mut state, &mut Vec::new(), P1, 1, &[P1]);
        assert_eq!(
            card_at(&state, slot(P1, Row::Backrow, 2)).map(|card| card.id.clone()),
            Some(trap_id.clone())
        );
        assert!(card_at(&state, slot(P1, Row::Backrow, 4)).is_none());
        assert!(find_instance(&state, &first).is_none());
    }

    #[test]
    fn r227_r97_a_card_going_back_face_down_from_a_public_zone_takes_a_fresh_id_its_views_follow_it_by_former_id() {
        let mut state = board("bh-fresh");
        let trap = put(&mut state, &watcher().id, slot(P2, Row::Backrow, 1));
        record_board_snapshot(&mut state);
        move_to_zone(&mut state, &trap.id, OffFieldZone::Graveyard, MoveToZoneOptions::default());
        let old = trap.id.clone();
        let mut events: Vec<GameEvent> = Vec::new();
        restore(&mut state, &mut events, P1, 1, &[P2]);
        let back = card_at(&state, slot(P2, Row::Backrow, 1)).cloned();
        assert_ne!(back.as_ref().map(|card| card.id.clone()), Some(old.clone()));
        let back_id = back.as_ref().map(|card| card.id.clone());
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::ControlChanged)),
            json!([{ "type": "controlChanged", "instanceId": back_id, "controller": "p2", "row": "backrow", "lane": 1, "formerId": old }])
        );
        state.applied.push(AppliedAction {
            nonce: "bh-fresh".into(),
            events: events.clone(),
        });
        let seen = json_of(events_of_type(&view_for(&state, P1).events, GameEventType::ControlChanged));
        assert_eq!(
            seen,
            json!([{ "type": "controlChanged", "instanceId": "hidden", "controller": "p2", "row": "backrow", "lane": 1 }])
        );
        let theirs = view_for(&state, P2);
        let theirs = events_of_type(&theirs.events, GameEventType::ControlChanged);
        assert_eq!(theirs.first().map(|event| json_of(event)["formerId"].clone()), Some(json!(old)));
    }

    #[test]
    fn r566_r227_a_face_up_trap_going_back_face_down_on_its_own_side_takes_a_fresh_id_it_has_entered_and_control_changed_names_it()
     {
        let mut state = board("bh-face-up");
        let trap = put(&mut state, &watcher().id, slot(P1, Row::Backrow, 2));
        let turn = state.turn;
        card_mut(&mut state, &trap.id).summoned_turn = Some(turn - 2);
        record_board_snapshot(&mut state);
        card_mut(&mut state, &trap.id).face_up = Some(true);
        let old = trap.id.clone();
        let mut events: Vec<GameEvent> = Vec::new();
        restore(&mut state, &mut events, P1, 1, &[P1]);
        let back = card_at(&state, slot(P1, Row::Backrow, 2)).cloned().expect("a card went back");
        assert_ne!(back.id, old);
        assert_eq!(back.face_up, None);
        assert_eq!(back.summoned_turn, Some(state.turn));
        assert!(find_instance(&state, &old).is_none());
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::ControlChanged)),
            json!([{ "type": "controlChanged", "instanceId": back.id, "controller": "p1", "row": "backrow", "lane": 2, "formerId": old }])
        );
    }

    #[test]
    fn r566_r385_the_snapshot_holds_a_brittle_count_as_the_turn_began_before_its_tick_and_a_restored_card_takes_that_count()
     {
        let mut state = playing("bh-brittle");
        let unit = put(&mut state, PLAIN_UNIT, slot(P1, Row::Units, 1));
        let turn = state.turn;
        card_mut(&mut state, &unit.id).brittle = Some(BrittleCounter {
            count: 3,
            since: turn,
            printed: None,
        });
        state = act(&state, input(json!({ "type": "endTurn", "playerId": "p1" })));
        state = act(&state, input(json!({ "type": "endTurn", "playerId": "p2" })));
        // Turn 3, p1's: recorded first, then the tick (R62) took the live count down.
        assert_eq!(card(&state, &unit.id).brittle.map(|brittle| brittle.count), Some(2));
        let recorded = state
            .board_history
            .as_ref()
            .and_then(|history| history.last())
            .and_then(|snapshot| snapshot.sides.p1.units[0].as_ref())
            .and_then(|pile| pile.first())
            .and_then(|card| card.brittle);
        assert_eq!(
            recorded,
            Some(BrittleCounter {
                count: 3,
                since: 1,
                printed: None
            })
        );

        restore(&mut state, &mut Vec::new(), P1, 1, &[P1]);
        assert_eq!(
            card(&state, &unit.id).brittle,
            Some(BrittleCounter {
                count: 3,
                since: 1,
                printed: None
            })
        );
    }

    #[test]
    fn r566_r386_a_restored_card_takes_the_snapshots_tuning_cost_change_face_and_memory_a_degrade_or_a_make_radiant_made_since_off_the_field_is_undone()
     {
        let mut state = board("bh-tuning");
        let unit = put(&mut state, PLAIN_UNIT, slot(P1, Row::Units, 1));
        {
            let live = card_mut(&mut state, &unit.id);
            live.tuning = Some(json_as(json!({ "attack": 1 })));
            live.memory = IndexMap::from([("meal".to_string(), json!("kept"))]);
        }
        record_board_snapshot(&mut state);
        move_to_zone(&mut state, &unit.id, OffFieldZone::Hand, MoveToZoneOptions::default());
        // Since, in the hand (B3.4, §6.3): what R78 leaves alone on the way out, the restore still takes back.
        {
            let live = card_mut(&mut state, &unit.id);
            live.tuning = Some(json_as(json!({ "attack": -1, "health": -1 })));
            live.cost_mod = 1;
            live.radiant = true;
            live.memory = IndexMap::new();
        }
        restore(&mut state, &mut Vec::new(), P1, 1, &[P1]);
        assert!(matches_object(
            &json_of(card(&state, &unit.id)),
            &json!({ "zone": { "z": "field" }, "tuning": { "attack": 1 }, "costMod": 0, "radiant": false, "memory": { "meal": "kept" } })
        ));
    }

    #[test]
    fn r566_a_card_mid_play_stays_its_plays_and_its_place_stays_empty_a_unit_it_carried_left_with_no_carrier_goes_to_its_owners_hand()
     {
        let mut state = board("bh-resolving");
        let carrier = put(&mut state, &tower().id, slot(P1, Row::Backrow, 1));
        let rider = new_instance(&mut state, "fx-6", P1, Zone::Hand { player: P1 });
        place_on_field(
            &mut state,
            rider.clone(),
            slot(P1, Row::Backrow, 1),
            PlaceOnFieldOptions::default(),
        );
        record_board_snapshot(&mut state);
        // Since: both went back to the hand, and the carrier is being played again (§10.5's resolving zone).
        move_to_zone(&mut state, &rider.id, OffFieldZone::Hand, MoveToZoneOptions::default());
        let mut resolving = card(&state, &carrier.id).clone();
        remove_from_any_zone(&mut state, &carrier.id);
        resolving.zone = Zone::Resolving { player: P1 };
        state.players.p1.resolving.push(resolving);

        let mut events: Vec<GameEvent> = Vec::new();
        restore(&mut state, &mut events, P1, 1, &[P1]);
        assert_eq!(card(&state, &carrier.id).zone.z(), ZoneName::Resolving);
        assert!(zone_contents(&state, slot(P1, Row::Backrow, 1)).is_empty());
        assert_eq!(card(&state, &rider.id).zone.z(), ZoneName::Hand);
        assert_eq!(
            state.players.p1.hand.iter().filter(|held| held.id == rider.id).count(),
            1
        );
        assert_eq!(
            instance_ids(&events_of_type(&events, GameEventType::Bounced)),
            vec![json!(rider.id)]
        );
    }

    #[test]
    fn r566_a_card_that_stayed_on_its_side_keeps_its_exertion_and_sickness_one_put_back_from_elsewhere_entered_on_this_turn()
     {
        let mut state = board("bh-turn-state");
        let stayed = put(&mut state, PLAIN_UNIT, slot(P1, Row::Units, 1));
        let away = put(&mut state, "fx-2", slot(P1, Row::Units, 2));
        record_board_snapshot(&mut state);
        card_mut(&mut state, &stayed.id).exertion = Exertion {
            attacked: true,
            switched: false,
            attacks: None,
        };
        move_to_zone(&mut state, &away.id, OffFieldZone::Hand, MoveToZoneOptions::default());
        let mut events: Vec<GameEvent> = Vec::new();
        restore(&mut state, &mut events, P1, 1, &[P1]);
        assert!(card(&state, &stayed.id).exertion.attacked);
        let back = find_instance(&state, &away.id);
        assert_eq!(back.and_then(|card| card.summoned_turn), Some(state.turn));
        assert_eq!(
            instance_ids(&events_of_type(&events, GameEventType::ControlChanged)),
            vec![json!(away.id)]
        );
    }
}

mod r563_e29_held_zones {
    use super::*;

    #[test]
    fn r563_a_reborn_unit_whose_zone_a_rollback_let_go_does_not_return_the_snapshots_occupant_stands_there() {
        let mut state = board("bh-reborn");
        state.turn = 2;
        let occupant = put(&mut state, PLAIN_UNIT, slot(P1, Row::Units, 1));
        record_board_snapshot(&mut state);
        move_to_zone(&mut state, &occupant.id, OffFieldZone::Hand, MoveToZoneOptions::default());
        let bird = put(&mut state, &phoenix().id, slot(P1, Row::Units, 1));
        card_mut(&mut state, &bird.id).damage = 2;

        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            state_check(&mut sink);
        }
        assert_eq!(events_of_type(&events, GameEventType::RolledBack).len(), 1);
        assert_eq!(
            ids_of(&zone_contents(&state, slot(P1, Row::Units, 1))),
            vec![occupant.id.clone()]
        );
        assert_eq!(card(&state, &bird.id).zone.z(), ZoneName::Graveyard);
        assert!(state.reserved.is_empty());
    }

    #[test]
    fn r563_only_a_restored_sides_holds_are_let_go_the_other_sides_reborn_zone_and_animated_cards_home_stay_held() {
        let mut state = board("bh-holds");
        let animated = put(&mut state, &spatula().id, slot(P1, Row::Units, 2));
        record_board_snapshot(&mut state);
        reserve_home(&mut state, slot(P1, Row::Backrow, 2), &animated.id);
        reserve_zone(&mut state, slot(P1, Row::Units, 3));
        reserve_zone(&mut state, slot(P2, Row::Units, 3));

        restore(&mut state, &mut Vec::new(), P2, 1, &[P2]);
        assert_eq!(
            json_of(&state.reserved),
            json!([{ "player": "p1", "row": "units", "lane": 3 }])
        );
        assert_eq!(
            json_of(&state.homes),
            json!([{ "instanceId": animated.id, "zone": { "player": "p1", "row": "backrow", "lane": 2 } }])
        );
    }
}

mod e29_through_reduce_s9_3 {
    use super::*;

    fn step(state: &mut GameState, log: &mut Vec<Action>, body: ActionInput) {
        let action = body.with_nonce(format!("bh-{}", log.len()));
        let result = reduce(state, &action);
        if let Some(error) = result.error {
            panic!("{error}");
        }
        log.push(action);
        *state = result.state;
    }

    #[test]
    fn r419_a_game_that_rolls_back_survives_json_and_folds_to_the_same_hash_from_its_log() {
        // TS leaned on the catalog an earlier test had registered (the vanilla fixtures); each Rust test
        // starts on a fresh thread, so it registers them itself first.
        setup_catalog();
        register_board_history_fixtures();
        let mut first_deck = vec![rewind().id];
        first_deck.extend(vanilla_deck(DECK_SIZE - 1, 1));
        let decks = (first_deck, vanilla_deck(DECK_SIZE, 21));
        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: "bh-fold".into(),
            decks: decks.clone(),
            ..CreateGameOptions::default()
        }))
        .state;
        let mut log: Vec<Action> = Vec::new();
        let keep_p1: Vec<String> = state.players.p1.hand.iter().map(|card| card.id.clone()).collect();
        step(
            &mut state,
            &mut log,
            input(json!({ "type": "mulligan", "keep": keep_p1, "playerId": "p1" })),
        );
        let keep_p2: Vec<String> = state.players.p2.hand.iter().map(|card| card.id.clone()).collect();
        step(
            &mut state,
            &mut log,
            input(json!({ "type": "mulligan", "keep": keep_p2, "playerId": "p2" })),
        );
        let rewind_id = rewind().id;
        while state.turn < 5
            || state.active != P1
            || !state.players.p1.hand.iter().any(|card| card.def_id == rewind_id)
        {
            let player = state.active;
            let play = legal_actions(&state, player).into_iter().find(|action| {
                matches!(action, ActionBody::Play { instance_id, .. }
                    if find_instance(&state, instance_id).map(|card| card.def_id.as_str()) != Some(rewind_id.as_str()))
            });
            if let Some(play) = play {
                step(
                    &mut state,
                    &mut log,
                    ActionInput {
                        body: play,
                        player_id: player,
                    },
                );
            }
            step(
                &mut state,
                &mut log,
                input(json!({ "type": "endTurn", "playerId": player })),
            );
            if state.turn > 20 {
                panic!("p1 never held the fixture");
            }
        }
        let round = round_trip(&state);
        assert_eq!(hash_state(&round), hash_state(&state));
        let held = state
            .players
            .p1
            .hand
            .iter()
            .find(|card| card.def_id == rewind_id)
            .map(|card| card.id.clone())
            .unwrap_or_default();
        step(
            &mut state,
            &mut log,
            input(json!({ "type": "play", "playerId": "p1", "instanceId": held, "modes": ["2"] })),
        );
        assert!(state.players.p1.graveyard.iter().any(|card| card.def_id == rewind_id));

        let replayed = fold(&FoldArgs {
            seed: "bh-fold".into(),
            decks,
            log,
            ..FoldArgs::default()
        });
        assert!(replayed.errors.is_empty());
        assert_eq!(hash_state(&replayed.state), hash_state(&state));
        assert!(!serde_json::to_string(&view_for(&state, P2)).unwrap().contains("boardHistory"));
    }
}
