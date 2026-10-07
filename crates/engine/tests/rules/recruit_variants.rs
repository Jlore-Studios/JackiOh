//! The Recruit extensions of patch v0.2.0 (docs/classic-sets.md B5 E25; §6.3 Recruit, R11, R12, R64,
//! R218): from the opponent's exile newest first under your control, N at once, with filters, and
//! "your entire deck" — each permanent until its row is full, Spells staying.
//!
//! Port of `packages/engine/test/recruit-variants.test.ts`. TS's `sinkFor(state)` sink is kept for the
//! test's length so its events can be read after the run; the rng cursor is not written back, as TS
//! did not.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use jackioh_engine::effects::{forced_attacks, recruit, recruit_all};

use crate::rules::fixtures::generation::{
    Run, act, answer, asker, asker_answers, body, cheap_unit, clear_asker_answers, deck_spell, field_trap, frozen,
    hand_card, pile_on, plain_trap, playing, pricy_unit, replayed, x_unit,
};
use crate::rules::fixtures::harness::{events_of_type, put, set_library, sink_for, slot};

fn run(sink: &mut EngineSink<'_>, effects: Vec<Effect>) {
    let mut ctx = make_context(
        sink,
        None,
        HookOptions {
            controller: Some(P1),
            ..HookOptions::default()
        },
    );
    for effect in &effects {
        (effect.apply)(&mut ctx);
    }
}

/// TS `run(sinkFor(state), effects)` when the test reads no event.
fn run_on(state: &mut GameState, effects: Vec<Effect>) {
    let mut sink = sink_for(state);
    run(&mut sink, effects);
}

/// The ids of a pile, read now (a Recruit splices the very array `setLibrary` returned).
fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn must<T>(value: Option<T>, what: &str) -> T {
    value.unwrap_or_else(|| panic!("expected {what}"))
}

/// A card put into a player's exile, as the last one exiled.
fn exiled(state: &mut GameState, def_id: &str, owner: PlayerId) -> CardInstance {
    let card = new_instance(state, def_id, owner, Zone::Exile { player: owner });
    state.players[owner].exile.push(card.clone());
    card
}

fn top_of(state: &GameState, player: PlayerId, lane: usize) -> Option<CardInstance> {
    state.players[player].units[lane]
        .as_ref()
        .and_then(|pile| pile.first())
        .cloned()
}

/// TS `units.flatMap((pile) => (pile === null ? [] : [pile[0]?.id]))`.
fn unit_ids(side: &PlayerState) -> Vec<Option<String>> {
    side.units
        .iter()
        .flatten()
        .map(|pile| pile.first().map(|card| card.id.clone()))
        .collect()
}

fn args<T: serde::de::DeserializeOwned>(literal: Value) -> T {
    json_as(literal)
}

/// Jest's `toMatchObject`: every key `expected` names is in `actual` and matches, recursively; an
/// array matches element by element and in length.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, want)| actual.get(key).is_some_and(|got| matches_object(got, want))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len() && actual.iter().zip(expected).all(|(got, want)| matches_object(got, want))
        }
        _ => actual == expected,
    }
}

fn face_down_marker() -> BackrowView {
    BackrowView::FaceDown(FaceDownBackrowView {
        face_down: true,
        cost: Some(1),
        plague: None,
        buried: None,
        marks: None,
    })
}

/// E25 Recruit from the opponent's exile (Classic #1 Radiant)
mod e25_recruit_from_the_opponent_s_exile_classic_c1_radiant {
    use super::*;

    #[test]
    fn r12_the_newest_permanent_of_their_exile_is_summoned_on_your_side_under_your_control_its_owner_unchanged() {
        let mut start = playing("recruit-exile");
        let state = &mut start.state;
        let older = exiled(state, &pricy_unit.id, P2);
        exiled(state, &deck_spell.id, P2);
        let newest = exiled(state, &cheap_unit.id, P2);
        exiled(state, &deck_spell.id, P2); // a Spell exiled last is passed over
        let events = {
            let mut sink = sink_for(state);
            run(&mut sink, vec![recruit(args(json!({ "from": "exile", "whose": "enemy" })))]);
            sink.events.clone()
        };

        let mut card = must(top_of(state, P1, 0), "the recruited card");
        assert_eq!(card.id, newest.id);
        assert_eq!(card.owner, P2);
        assert_eq!(card.controller, P1);
        assert!(!ids(&state.players.p2.exile).contains(&newest.id));
        assert!(ids(&state.players.p2.exile).contains(&older.id));
        let summoned = serde_json::to_value(events_of_type(&events, GameEventType::Summoned)).expect("serialises");
        assert!(
            matches_object(&summoned, &json!([{ "player": "p1", "instanceId": newest.id }])),
            "{summoned}"
        );

        // It goes back to its owner's piles when it leaves the field (§3.2).
        move_to_zone(state, &mut card, OffFieldZone::Graveyard, Default::default());
        assert!(ids(&state.players.p2.graveyard).contains(&newest.id));
    }

    #[test]
    fn r53_if_it_s_a_unit_it_attacks_them_at_once_the_recruit_is_one_of_the_units_this_list_summoned() {
        let mut start = playing("recruit-exile-attack");
        let state = &mut start.state;
        exiled(state, &cheap_unit.id, P2);
        let before = state.players.p2.hero.health;
        run_on(
            state,
            vec![
                recruit(args(json!({ "from": "exile", "whose": "enemy" }))),
                forced_attacks(args(json!({
                    "attackers": { "summonedThisScript": true },
                    "target": { "spec": { "of": "enemyHero" } },
                }))),
            ],
        );
        assert_eq!(state.players.p2.hero.health, before - 1);
    }

    #[test]
    fn e25_an_exile_with_no_permanent_recruits_nothing() {
        let mut start = playing("recruit-exile-empty");
        let state = &mut start.state;
        exiled(state, &deck_spell.id, P2);
        let mut sink = sink_for(state);
        run(&mut sink, vec![recruit(args(json!({ "from": "exile", "whose": "enemy" })))]);
        assert_eq!(*sink.events, Vec::<GameEvent>::new());
    }
}

/// E25 Recruit N, with filters (Classic #31, #65)
mod e25_recruit_n_with_filters_classic_c31_c65 {
    use super::*;

    #[test]
    fn r64_n_scans_each_the_first_matching_permanent_from_the_top_cost_2_or_less_units_three_of_them() {
        let mut start = playing("recruit-count");
        let state = &mut start.state;
        let library = ids(&set_library(
            state,
            P1,
            &[
                pricy_unit.id.clone(),
                cheap_unit.id.clone(),
                deck_spell.id.clone(),
                body.id.clone(),
                plain_trap.id.clone(),
                cheap_unit.id.clone(),
            ],
        ));
        run_on(
            state,
            vec![recruit(args(json!({ "count": 3, "filter": { "type": "Unit", "costRange": { "max": 2 } } })))],
        );
        assert_eq!(
            unit_ids(&state.players.p1),
            vec![Some(library[1].clone()), Some(library[3].clone()), Some(library[5].clone())]
        );
        assert_eq!(
            ids(&state.players.p1.library),
            vec![library[0].clone(), library[2].clone(), library[4].clone()]
        );
    }

    #[test]
    fn r64_a_scan_whose_card_finds_no_zone_fizzles_and_the_next_finds_the_same_card_core_c69_s_shape() {
        let mut start = playing("recruit-count-full");
        let state = &mut start.state;
        for lane in 1..=5 {
            put(state, &body.id, slot(P1, Row::Units, lane), json!({}));
        }
        let library = ids(&set_library(state, P1, &[cheap_unit.id.clone(), plain_trap.id.clone()]));
        let events = {
            let mut sink = sink_for(state);
            run(&mut sink, vec![recruit(args(json!({ "count": 2 })))]);
            sink.events.clone()
        };
        assert_eq!(events_of_type(&events, GameEventType::Summoned), Vec::<GameEvent>::new());
        assert_eq!(ids(&state.players.p1.library), library);
    }
}

/// E25 Recruit your entire deck (Classic #60)
mod e25_recruit_your_entire_deck_classic_c60 {
    use super::*;

    #[test]
    fn r64_top_down_each_permanent_while_its_row_has_room_units_to_units_the_rest_to_the_backrow_traps_face_down_spells_stay() {
        let mut start = playing("recruit-all");
        let state = &mut start.state;
        for lane in 1..=3 {
            put(state, &body.id, slot(P1, Row::Units, lane), json!({}));
        }
        let library = ids(&set_library(
            state,
            P1,
            &[
                cheap_unit.id.clone(),
                deck_spell.id.clone(),
                plain_trap.id.clone(),
                pricy_unit.id.clone(),
                body.id.clone(), // the unit row is full by now: this one stays
                field_trap.id.clone(),
            ],
        ));
        let spell = hand_card(state, &pile_on.id, P1);
        state.players.p1.mana.current = 5;
        let mut run1 = frozen(&start);
        run1 = act(&run1, json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }));
        let after = &run1.state.players.p1;
        assert_eq!(
            after
                .units
                .iter()
                .map(|pile| pile.as_ref().and_then(|pile| pile.first()).map(|card| card.def_id.clone()))
                .collect::<Vec<_>>(),
            vec![
                Some(body.id.clone()),
                Some(body.id.clone()),
                Some(body.id.clone()),
                Some(cheap_unit.id.clone()),
                Some(pricy_unit.id.clone()),
            ]
        );
        let traps: Vec<&CardInstance> = after.backrow.iter().flatten().collect();
        assert_eq!(
            traps.iter().map(|card| card.def_id.clone()).collect::<Vec<_>>(),
            vec![plain_trap.id.clone(), field_trap.id.clone()]
        );
        assert!(traps.iter().all(|card| card.face_up != Some(true)));
        assert_eq!(ids(&after.library), vec![library[1].clone(), library[4].clone()]);
        assert_eq!(hash_state(&replayed(&run1)), hash_state(&run1.state));
        // A Trap it set is face-down to the other player (R33, R227).
        assert_eq!(
            view_for(&run1.state, P2)
                .opponent
                .backrow
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
            vec![face_down_marker(), face_down_marker()]
        );
    }

    #[test]
    fn r113_a_card_that_asks_a_question_as_it_arrives_pauses_the_rest_which_resumes_over_the_same_cards_after_the_answer() {
        let mut start = playing("recruit-all-pause");
        let state = &mut start.state;
        let library = ids(&set_library(
            state,
            P1,
            &[asker.id.clone(), cheap_unit.id.clone(), deck_spell.id.clone(), body.id.clone()],
        ));
        let spell = hand_card(state, &pile_on.id, P1);
        state.players.p1.mana.current = 5;
        clear_asker_answers();
        let mut run1 = frozen(&start);
        run1 = act(&run1, json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }));

        // The Field Spell arrived and asked; the two Units behind it wait (R151, R113).
        let pending = must(run1.state.pending.clone(), "the arrival's question");
        assert_eq!(pending.player_id, P1);
        assert!(run1.state.players.p1.units.iter().all(Option::is_none));
        let round: GameState =
            serde_json::from_str(&serde_json::to_string(&run1.state).expect("serialises")).expect("parses");
        assert_eq!(round, run1.state);

        let right = || Selection::Mode {
            option: "right".to_string(),
        };
        let live_run = answer(&run1, right(), None);
        let from_json = answer(
            &Run {
                state: round,
                ..run1.clone()
            },
            right(),
            None,
        );
        assert_eq!(hash_state(&from_json.state), hash_state(&live_run.state));
        assert_eq!(hash_state(&replayed(&live_run)), hash_state(&live_run.state));
        assert!(asker_answers().contains(&"right".to_string()));
        assert_eq!(
            unit_ids(&live_run.state.players.p1),
            vec![Some(library[1].clone()), Some(library[3].clone())]
        );
        assert_eq!(ids(&live_run.state.players.p1.library), vec![library[2].clone()]);
        let asker_card = must(find_instance(&live_run.state, &library[0]), "the asker");
        assert!(
            matches!(asker_card.zone, Zone::Field { row: Row::Backrow, .. }),
            "{:?}",
            asker_card.zone
        );
    }

    #[test]
    fn e25_recruit_all_reaches_the_opponent_s_exile_as_well_newest_first_and_a_filter_narrows_it() {
        let mut start = playing("recruit-all-exile");
        let state = &mut start.state;
        let first = exiled(state, &pricy_unit.id, P2);
        let second = exiled(state, &cheap_unit.id, P2);
        exiled(state, &deck_spell.id, P2);
        run_on(
            state,
            vec![recruit_all(args(json!({ "from": "exile", "whose": "enemy", "filter": { "costRange": { "max": 1 } } })))],
        );
        assert_eq!(top_of(state, P1, 0).map(|card| card.id), Some(second.id.clone()));
        assert!(state.players.p1.units[1].is_none());
        assert!(ids(&state.players.p2.exile).contains(&first.id));
    }
}

/// R690 Recruit skips (X)-cost cards unless they are the only valid targets
mod r690_recruit_skips_x_cost_cards_unless_they_are_the_only_valid_targets {
    use super::*;

    #[test]
    fn r690_a_scan_takes_the_first_non_x_match_past_an_x_cost_card_on_top_which_stays() {
        let mut start = playing("recruit-x-skip");
        let state = &mut start.state;
        let library = ids(&set_library(state, P1, &[x_unit.id.clone(), cheap_unit.id.clone()]));
        run_on(state, vec![recruit(args(json!({})))]);
        assert_eq!(top_of(state, P1, 0).map(|card| card.id), Some(library[1].clone()));
        assert_eq!(ids(&state.players.p1.library), vec![library[0].clone()]);
    }

    #[test]
    fn r690_with_only_x_cost_matches_the_scan_takes_the_first_one() {
        let mut start = playing("recruit-x-only");
        let state = &mut start.state;
        let library = ids(&set_library(state, P1, &[x_unit.id.clone(), deck_spell.id.clone()]));
        run_on(state, vec![recruit(args(json!({})))]);
        assert_eq!(top_of(state, P1, 0).map(|card| card.id), Some(library[0].clone()));
        assert_eq!(ids(&state.players.p1.library), vec![library[1].clone()]);
    }

    #[test]
    fn r690_recruit_all_leaves_x_cost_cards_when_other_permanents_match_and_takes_them_when_nothing_else_does() {
        let mut mixed = playing("recruit-all-x-mixed");
        let mixed_state = &mut mixed.state;
        let mixed_library = ids(&set_library(
            mixed_state,
            P1,
            &[cheap_unit.id.clone(), x_unit.id.clone(), deck_spell.id.clone()],
        ));
        run_on(mixed_state, vec![recruit_all(args(json!({})))]);
        assert_eq!(top_of(mixed_state, P1, 0).map(|card| card.id), Some(mixed_library[0].clone()));
        assert_eq!(
            ids(&mixed_state.players.p1.library),
            vec![mixed_library[1].clone(), mixed_library[2].clone()]
        );

        let mut only = playing("recruit-all-x-only");
        let only_state = &mut only.state;
        let only_library = ids(&set_library(only_state, P1, &[x_unit.id.clone(), deck_spell.id.clone()]));
        run_on(only_state, vec![recruit_all(args(json!({})))]);
        assert_eq!(top_of(only_state, P1, 0).map(|card| card.id), Some(only_library[0].clone()));
        assert_eq!(ids(&only_state.players.p1.library), vec![only_library[1].clone()]);
    }
}
