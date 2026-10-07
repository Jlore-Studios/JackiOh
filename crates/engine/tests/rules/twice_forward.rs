//! C+ #74 Twice Forward One Step Backwards' Field Trap (subsystems/twiceForward.ts, R425): it counts the
//! opponent's plays from when it is set, and on every second one, once that card has resolved, fuses it
//! (Radiant: a Radiant copy of it) into itself (R77, R102) and gains Brittle (R385); with nothing left to
//! fuse it gains the Brittle face-down; it turns face-up at its first fuse (R33). Through fixture scripts
//! (fixtures/twiceForward.ts); the real card's test covers the same cases again.
//!
//! Port of `packages/engine/test/twiceForward.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::subsystems::twice_forward::{TWICE_FORWARD_PLAYS_KEY, twice_forward_plays};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, slot};
use crate::rules::fixtures::twice_forward::{
    CRIER_DAMAGE, TURNER_DAMAGE, TWICE_FORWARD_SCRIPTS, caster, crier, forward, self_exiler, spell, trap,
    turner, twice_forward_catalog,
};

/// TS's module-level `let nonce`.
static NONCE: AtomicU32 = AtomicU32::new(0);

/// What `act` hands back: the state and the events of the action.
struct Acted {
    state: GameState,
    events: Vec<GameEvent>,
}

/// `body` is the TS `ActionInput` literal; the nonce is added here.
fn act(state: &GameState, body: Value) -> Acted {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let mut action = body;
    action["nonce"] = json!(format!("tf{nonce}"));
    let result = reduce(state, &json_as::<Action>(action));
    if let Some(error) = result.error {
        panic!("{error}");
    }
    Acted {
        state: result.state,
        events: result.events,
    }
}

fn hand_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player]
        .hand
        .iter()
        .map(|card| card.id.clone())
        .collect()
}

/// `opponentsTurn`'s answer.
struct OpponentsTurn {
    state: GameState,
    trap_id: String,
}

/// p1 has the trap set face-down (lane 2) on its turn; then p2's turn begins, with mana to spare.
fn opponents_turn(seed: &str, radiant: bool) -> OpponentsTurn {
    let mut state = begin_game(&new_game(&format!("twice-forward-{seed}"), None)).state;
    register_catalog(twice_forward_catalog(registered_catalog().clone()));
    let mut registry = registered_scripts().clone();
    for (id, script) in TWICE_FORWARD_SCRIPTS.clone() {
        registry.insert(id, script);
    }
    register_scripts(registry);
    for player in [PlayerId::P1, PlayerId::P2] {
        state = act(
            &state,
            json!({ "type": "mulligan", "keep": hand_ids(&state, player), "playerId": player }),
        )
        .state;
    }
    state.players.p1.auto_end_turn = Some(false);
    state.players.p2.auto_end_turn = Some(false);
    let held = in_hand(&mut state, &forward().id, PlayerId::P1, 1)
        .into_iter()
        .next()
        .expect("no trap");
    find_instance_mut(&mut state, &held.id).expect("no trap").radiant = radiant;
    state.players.p1.mana.current = 4;
    state = act(
        &state,
        json!({
            "type": "play",
            "instanceId": held.id,
            "zone": { "row": "backrow", "lane": 2 },
            "playerId": "p1",
        }),
    )
    .state;
    // R227: a card set face-down takes a fresh id.
    let trap_id = state.players.p1.backrow[1]
        .as_ref()
        .map(|card| card.id.clone())
        .unwrap_or_default();
    state = act(&state, json!({ "type": "endTurn", "playerId": "p1" })).state;
    state.players.p2.mana.current = 10;
    OpponentsTurn { state, trap_id }
}

/// What `play` hands back: the action's state and events, and the card that was played.
struct Played {
    state: GameState,
    events: Vec<GameEvent>,
    card: CardInstance,
}

/// TS `play(state, player, defId, zone?)`: the card goes into `player`'s hand and is played.
fn play(mut state: GameState, player: PlayerId, def_id: &str, zone: Option<(Row, i32)>) -> Played {
    let card = in_hand(&mut state, def_id, player, 1)
        .into_iter()
        .next()
        .expect("no card in hand");
    let mut body = json!({ "type": "play", "instanceId": card.id, "playerId": player });
    if let Some((row, lane)) = zone {
        body["zone"] = json!({ "row": row, "lane": lane });
    }
    let acted = act(&state, body);
    Played {
        state: acted.state,
        events: acted.events,
        card,
    }
}

fn trap_of<'a>(state: &'a GameState, trap_id: &str) -> &'a CardInstance {
    find_instance(state, trap_id).expect("the trap is gone")
}

/// The trap, to write through as TS wrote through the live object `trapOf` handed back.
fn trap_of_mut<'a>(state: &'a mut GameState, trap_id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, trap_id).expect("the trap is gone")
}

/// `eventsOfType(events, "fused")[0]?.instanceIds`.
fn first_fused_ids(events: &[GameEvent]) -> Option<Vec<String>> {
    events_of_type(events, GameEventType::Fused)
        .iter()
        .find_map(|event| match event {
            GameEvent::Fused { instance_ids, .. } => Some(instance_ids.clone()),
            _ => None,
        })
}

/// `eventsOfType(events, "cardPlayed")` as `(defId, instanceId)` pairs.
fn plays_of(events: &[GameEvent]) -> Vec<(String, String)> {
    events_of_type(events, GameEventType::CardPlayed)
        .iter()
        .filter_map(|event| match event {
            GameEvent::CardPlayed {
                def_id, instance_id, ..
            } => Some((def_id.clone(), instance_id.clone())),
            _ => None,
        })
        .collect()
}

/// `types.indexOf(type)`: -1 when absent, as in TS.
fn index_of(types: &[GameEventType], kind: GameEventType) -> i64 {
    types
        .iter()
        .position(|each| *each == kind)
        .map_or(-1, |index| index as i64)
}

/// TS `JSON.parse(JSON.stringify(x))`.
fn round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
    serde_json::from_value(serde_json::to_value(value).expect("serialises")).expect("deserialises")
}

mod c_plus_c74s_field_trap_r425 {
    use super::*;

    #[test]
    fn r687_set_face_down_it_holds_no_brittle_only_its_controller_reads_the_back() {
        let OpponentsTurn { state, trap_id } = opponents_turn("brittle", false);
        let card = trap_of(&state, &trap_id);
        assert!(card.face_up != Some(true));
        assert_eq!(active_brittle_count(card), None);
        assert_eq!(card.brittle, None);
    }

    #[test]
    fn r425_r99_counts_the_opponents_1st_play_and_stays_face_down_unfired() {
        let OpponentsTurn { state, trap_id } = opponents_turn("first", false);
        let after = play(state, PlayerId::P2, &spell().id, None);
        assert_eq!(twice_forward_plays(trap_of(&after.state, &trap_id)), 1);
        assert!(trap_of(&after.state, &trap_id).face_up != Some(true));
        assert!(events_of_type(&after.events, GameEventType::TrapFired).is_empty());
    }

    #[test]
    fn r425_r77_on_the_2nd_after_it_resolves_a_unit_on_the_field_is_fused_into_this_still_a_field_trap_1_brittle_face_up()
     {
        let OpponentsTurn { state, trap_id } = opponents_turn("unit", false);
        let first = play(state, PlayerId::P2, &spell().id, None);
        let second = play(first.state, PlayerId::P2, "fx-5", Some((Row::Units, 1)));
        let kept = trap_of(&second.state, &trap_id);
        assert!(find_instance(&second.state, &second.card.id).is_none());
        assert!(second.state.players.p2.units[0].is_none());
        assert_eq!(
            def_of(Some(&second.state), &kept.def_id).type_,
            CardType::FieldTrap
        );
        assert_eq!(
            kept.zone,
            Zone::Field {
                player: PlayerId::P1,
                row: Row::Backrow,
                lane: 2,
            }
        );
        // R687: the first fuse reveals it and starts its printed Brittle 2, then gains +1.
        assert_eq!(active_brittle_count(kept), Some(3));
        assert_eq!(kept.face_up, Some(true));
        let types: Vec<GameEventType> = second.events.iter().map(GameEvent::event_type).collect();
        assert!(index_of(&types, GameEventType::CardResolved) < index_of(&types, GameEventType::TrapFired));
        assert!(index_of(&types, GameEventType::TrapFired) < index_of(&types, GameEventType::Fused));
    }

    #[test]
    fn r425_a_spell_is_fused_from_its_owners_graveyard() {
        let OpponentsTurn { state, trap_id } = opponents_turn("spell", false);
        let second = play(
            play(state, PlayerId::P2, &spell().id, None).state,
            PlayerId::P2,
            &spell().id,
            None,
        );
        assert!(
            !second
                .state
                .players
                .p2
                .graveyard
                .iter()
                .any(|card| card.id == second.card.id)
        );
        assert!(first_fused_ids(&second.events).is_some_and(|ids| ids.contains(&second.card.id)));
        assert_eq!(trap_of(&second.state, &trap_id).face_up, Some(true));
    }

    #[test]
    fn r425_a_trap_is_fused_from_its_owners_backrow() {
        let OpponentsTurn { state, trap_id } = opponents_turn("trap", false);
        let second = play(
            play(state, PlayerId::P2, &spell().id, None).state,
            PlayerId::P2,
            &trap().id,
            Some((Row::Backrow, 1)),
        );
        assert!(second.state.players.p2.backrow[0].is_none());
        // R227: the trap was set under a fresh id, the one its play announced.
        let set_id = plays_of(&second.events).first().map(|(_, id)| id.clone());
        assert!(
            first_fused_ids(&second.events)
                .is_some_and(|ids| set_id.as_ref().is_some_and(|set_id| ids.contains(set_id)))
        );
        assert_eq!(active_brittle_count(trap_of(&second.state, &trap_id)), Some(3));
    }

    #[test]
    fn r589_r425_r687_with_nothing_left_to_fuse_an_exiled_spell_it_reveals_and_still_gains_1_brittle_unfired()
    {
        let OpponentsTurn { state, trap_id } = opponents_turn("gone", false);
        let second = play(
            play(state, PlayerId::P2, &spell().id, None).state,
            PlayerId::P2,
            &self_exiler().id,
            None,
        );
        let kept = trap_of(&second.state, &trap_id);
        assert!(!second.card.id.is_empty());
        assert!(
            second
                .state
                .players
                .p2
                .exile
                .iter()
                .any(|card| card.id == second.card.id)
        );
        assert!(events_of_type(&second.events, GameEventType::TrapFired).is_empty());
        // No Brittle sits on an unrevealed card: the gain reveals it first, then lands on the started 2.
        assert_eq!(kept.face_up, Some(true));
        assert_eq!(kept.def_id, forward().id);
        assert_eq!(active_brittle_count(kept), Some(3));
        // Revealed, the opponent reads the card now.
        let theirs = serde_json::to_string(&view_for(&second.state, PlayerId::P2)).expect("serialises");
        assert!(theirs.contains(&trap_id));
        assert!(theirs.contains(&forward().id));
    }

    #[test]
    fn r70_r425_a_card_a_play_casts_is_the_later_play_the_cast_is_the_2nd_and_is_fused_not_the_card_that_cast_it()
     {
        let OpponentsTurn { state, trap_id } = opponents_turn("nested", false);
        let first = play(state, PlayerId::P2, &caster().id, None);
        let played = plays_of(&first.events);
        assert_eq!(
            played
                .iter()
                .map(|(def_id, _)| def_id.clone())
                .collect::<Vec<_>>(),
            vec![caster().id, spell().id]
        );
        assert_eq!(
            first_fused_ids(&first.events),
            Some(vec![
                played.get(1).map(|(_, id)| id.clone()).unwrap_or_default(),
                trap_id.clone(),
            ])
        );
        assert!(
            first
                .state
                .players
                .p2
                .graveyard
                .iter()
                .any(|card| card.def_id == caster().id)
        );
        assert_eq!(twice_forward_plays(trap_of(&first.state, &trap_id)), 2);
    }

    #[test]
    fn r425_its_own_controllers_plays_never_count() {
        let OpponentsTurn { state, trap_id } = opponents_turn("own", false);
        let mut next = act(&state, json!({ "type": "endTurn", "playerId": "p2" })).state;
        next.players.p1.mana.current = 10;
        next = play(next, PlayerId::P1, &spell().id, None).state;
        next = play(next, PlayerId::P1, &spell().id, None).state;
        assert_eq!(twice_forward_plays(trap_of(&next, &trap_id)), 0);
        assert!(trap_of(&next, &trap_id).face_up != Some(true));
    }

    #[test]
    fn r102_a_fused_end_of_turn_line_runs_for_its_controller_a_fused_cry_never_runs() {
        let OpponentsTurn { state, trap_id } = opponents_turn("texts", false);
        let mut next = play(state, PlayerId::P2, &turner().id, Some((Row::Units, 1))).state;
        next = play(next, PlayerId::P2, &turner().id, Some((Row::Units, 2))).state;
        // The second Turner is fused into p1's trap; the first stays p2's.
        assert_ne!(trap_of(&next, &trap_id).def_id, forward().id);
        let p2_before = next.players.p2.hero.health;
        let p1_before = next.players.p1.hero.health;
        next = act(&next, json!({ "type": "endTurn", "playerId": "p2" })).state;
        // p2's own Turner hit p1 at p2's end of turn; the fused one waits for p1's end of turn.
        assert_eq!(next.players.p1.hero.health, p1_before - TURNER_DAMAGE);
        next = act(&next, json!({ "type": "endTurn", "playerId": "p1" })).state;
        assert_eq!(next.players.p2.hero.health, p2_before - TURNER_DAMAGE);

        let cry = opponents_turn("cry", false);
        let crier_fused = play(
            play(cry.state, PlayerId::P2, &spell().id, None).state,
            PlayerId::P2,
            &crier().id,
            Some((Row::Units, 1)),
        );
        // The Crier's own Cry hit p1 as it was played; fused into p1's trap, its Cry never runs again.
        let crier_hits = events_of_type(&crier_fused.events, GameEventType::Damage)
            .iter()
            .filter(|event| matches!(event, GameEvent::Damage { amount, .. } if *amount == CRIER_DAMAGE))
            .count();
        assert_eq!(crier_hits, 1);
        assert_eq!(crier_fused.state.players.p2.hero.health, 30);
    }

    #[test]
    fn r425_it_goes_on_counting_every_second_play_fuses_again() {
        let OpponentsTurn { state, trap_id } = opponents_turn("again", false);
        let mut next = state;
        for _ in 0..4 {
            next = play(next, PlayerId::P2, &spell().id, None).state;
        }
        assert_eq!(twice_forward_plays(trap_of(&next, &trap_id)), 4);
        assert_eq!(active_brittle_count(trap_of(&next, &trap_id)), Some(4));
    }

    #[test]
    fn r386_the_every_n_step_reads_through_param_an_upgrades_plays_never_goes_below_2_a_degrade_makes_it_3() {
        let OpponentsTurn { mut state, trap_id } = opponents_turn("params", false);
        step_param(trap_of_mut(&mut state, &trap_id), "plays", -1);
        let mut next = play(
            play(state, PlayerId::P2, &spell().id, None).state,
            PlayerId::P2,
            &spell().id,
            None,
        )
        .state;
        assert_eq!(trap_of(&next, &trap_id).face_up, Some(true));

        let mut slow = opponents_turn("params-slow", false);
        step_param(trap_of_mut(&mut slow.state, &slow.trap_id), "plays", 1);
        step_param(trap_of_mut(&mut slow.state, &slow.trap_id), "brittleGain", 1);
        next = play(
            play(slow.state, PlayerId::P2, &spell().id, None).state,
            PlayerId::P2,
            &spell().id,
            None,
        )
        .state;
        assert!(trap_of(&next, &slow.trap_id).face_up != Some(true));
        next = play(next, PlayerId::P2, &spell().id, None).state;
        assert_eq!(trap_of(&next, &slow.trap_id).face_up, Some(true));
        assert_eq!(active_brittle_count(trap_of(&next, &slow.trap_id)), Some(4));
    }

    #[test]
    fn r179_its_count_and_fused_definition_survive_json_and_the_round_trip_plays_on_exactly_as_the_live_game()
    {
        let OpponentsTurn { state, trap_id } = opponents_turn("json", false);
        let mut fused = play(
            play(state, PlayerId::P2, &spell().id, None).state,
            PlayerId::P2,
            "fx-5",
            Some((Row::Units, 1)),
        )
        .state;
        let mut round: GameState = round_trip(&fused);
        assert_eq!(round, fused);
        assert_eq!(
            trap_of(&round, &trap_id).memory.get(TWICE_FORWARD_PLAYS_KEY),
            Some(&json!(2))
        );
        let a = in_hand(&mut fused, &spell().id, PlayerId::P2, 1)
            .into_iter()
            .next();
        let b = in_hand(&mut round, &spell().id, PlayerId::P2, 1)
            .into_iter()
            .next();
        let live = act(
            &fused,
            json!({
                "type": "play",
                "instanceId": a.map(|card| card.id).unwrap_or_default(),
                "playerId": "p2",
            }),
        )
        .state;
        let again = act(
            &round,
            json!({
                "type": "play",
                "instanceId": b.map(|card| card.id).unwrap_or_default(),
                "playerId": "p2",
            }),
        )
        .state;
        assert_eq!(hash_state(&again), hash_state(&live));
        assert_eq!(twice_forward_plays(trap_of(&again, &trap_id)), 3);
    }

    mod radiant {
        use super::*;

        #[test]
        fn r469_a_radiant_copy_of_the_played_card_is_fused_in_and_the_card_stays_where_it_is() {
            let OpponentsTurn { state, trap_id } = opponents_turn("radiant", true);
            let second = play(
                play(state, PlayerId::P2, &spell().id, None).state,
                PlayerId::P2,
                "fx-5",
                Some((Row::Units, 1)),
            );
            assert_eq!(
                second.state.players.p2.units[0]
                    .as_ref()
                    .and_then(|pile| pile.first())
                    .map(|card| card.id.clone()),
                Some(second.card.id.clone())
            );
            let kept = trap_of(&second.state, &trap_id);
            assert_ne!(kept.def_id, forward().id);
            assert_eq!(
                def_of(Some(&second.state), &kept.def_id).type_,
                CardType::FieldTrap
            );
            assert_eq!(active_brittle_count(kept), Some(5));
            assert_eq!(kept.face_up, Some(true));
        }

        #[test]
        fn r425_a_card_that_left_still_has_its_copy_fused_the_radiant_face_always_fuses() {
            let OpponentsTurn { state, trap_id } = opponents_turn("radiant-gone", true);
            let second = play(
                play(state, PlayerId::P2, &spell().id, None).state,
                PlayerId::P2,
                &self_exiler().id,
                None,
            );
            assert_eq!(events_of_type(&second.events, GameEventType::Fused).len(), 1);
            assert_eq!(trap_of(&second.state, &trap_id).face_up, Some(true));
            assert!(
                second
                    .state
                    .players
                    .p2
                    .exile
                    .iter()
                    .any(|card| card.id == second.card.id)
            );
        }
    }

    #[test]
    fn put_places_it_face_down_too_a_set_trap_counts_from_when_it_arrived() {
        let OpponentsTurn { mut state, .. } = opponents_turn("placed", false);
        let placed = put(
            &mut state,
            &forward().id,
            slot(PlayerId::P1, Row::Backrow, 4),
            json!({}),
        );
        trap_of_mut(&mut state, &placed.id).face_up = Some(false);
        let next = play(state, PlayerId::P2, &spell().id, None).state;
        assert_eq!(twice_forward_plays(trap_of(&next, &placed.id)), 1);
    }
}
