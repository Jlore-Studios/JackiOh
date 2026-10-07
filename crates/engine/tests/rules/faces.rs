//! Port of `packages/engine/test/faces.test.ts`.
//!
//! Faces that change more than text (docs/classic-sets.md B2.7, B5 E40): a face with its own type
//! (Classic+ #22 Blood Moon's Radiant face is a Field Trap) and X in the stats (Classic+ #69 Buff
//! Billy's "[3X/3X]"). Every rule that asks what an instance *is* reads the running face's type
//! (`faces.cardTypeOf`); a definition read on its own — a pool, a catalog filter — reads `def.type`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::effects::{cards_in_card_scope, summon};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::harness::{in_hand, put, slot};
use crate::rules::fixtures::instance_data::{billy, blood_moon, instance_game};

/// Jest's `toMatchObject` over serialised JSON: every key `expected` names is in `actual` with a
/// matching value (objects by subset, arrays element by element and of the same length).
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

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// The instance as the state holds it now (TS reads its live object).
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id).cloned().unwrap_or_else(|| panic!("{id} is in no zone"))
}

fn game() -> GameState {
    let mut state = instance_game("faces", None);
    state.turn = 3;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

/// `describe("B2.7 a face with its own type")`.
mod b2_7_a_face_with_its_own_type {
    use super::*;

    #[test]
    fn b2_7_the_card_s_type_is_its_running_face_s_a_radiant_blood_moon_is_a_field_trap_the_base_one_a_trap() {
        let mut state = game();
        let cards = in_hand(&mut state, &blood_moon.id, P1, 2);
        let (Some(base), Some(radiant)) = (cards.first().cloned(), cards.get(1).cloned()) else {
            panic!("no card");
        };
        find_instance_mut(&mut state, &radiant.id).expect("in hand").radiant = true;
        let radiant = live(&state, &radiant.id);
        assert_eq!(running_face(&state, &radiant).type_, Some(CardType::FieldTrap));
        assert_eq!(card_type_of(&state, &base), CardType::Trap);
        assert_eq!(card_type_of(&state, &radiant), CardType::FieldTrap);
        assert_eq!([is_trap_type(&state, &base), is_trap_type(&state, &radiant)], [true, true]);
        assert_eq!([is_field_trap(&state, &base), is_field_trap(&state, &radiant)], [false, true]);
        assert_eq!(
            [
                lands_face_down(&state, &base, Row::Backrow),
                lands_face_down(&state, &radiant, Row::Backrow)
            ],
            [true, true]
        );
        assert_eq!(row_for_card(&state, &radiant), Row::Backrow);
    }

    #[test]
    fn b2_7_a_radiant_blood_moon_stays_on_the_field_after_it_fires_as_a_field_trap_does_the_base_one_goes() {
        let mut state = game();
        let base = put(&mut state, &blood_moon.id, slot(P1, Row::Backrow, 1), json!({}));
        let radiant = put(&mut state, &blood_moon.id, slot(P1, Row::Backrow, 2), json!({ "radiant": true }));
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let base_now = live(sink.state, &base.id);
            consume_trap(&mut sink, &base_now);
            let radiant_now = live(sink.state, &radiant.id);
            consume_trap(&mut sink, &radiant_now);
        }
        assert_eq!(live(&state, &base.id).zone.z(), ZoneName::Graveyard);
        let radiant = live(&state, &radiant.id);
        assert_eq!(
            radiant.zone,
            Zone::Field {
                player: P1,
                row: Row::Backrow,
                lane: 2
            }
        );
        assert_eq!(radiant.face_up, Some(true));
    }

    #[test]
    fn b2_7_filters_read_the_card_s_type_now_a_pool_of_definitions_reads_the_definition_s() {
        let mut state = game();
        let cards = in_hand(&mut state, &blood_moon.id, P1, 1);
        let Some(radiant) = cards.first().cloned() else {
            panic!("no card");
        };
        find_instance_mut(&mut state, &radiant.id).expect("in hand").radiant = true;
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let sink = EngineSink::new(&mut state, &mut events, &mut rng);
        let ctx = make_context(
            sink,
            None,
            HookOptions {
                controller: Some(P1),
                ..Default::default()
            },
        );
        let field_traps: Vec<String> = cards_in_card_scope(
            &ctx,
            &json_as(json!({ "zones": ["hand"], "types": ["Field Trap"] })),
            Default::default(),
        )
        .iter()
        .map(|entry| entry.card.id.clone())
        .collect();
        assert_eq!(field_traps, vec![radiant.id.clone()]);
        assert!(
            cards_in_card_scope(&ctx, &json_as(json!({ "zones": ["hand"], "types": ["Trap"] })), Default::default())
                .is_empty()
        );
        let trap_ids: Vec<String> = query(json_as(json!({ "type": "Trap" }))).iter().map(|def| def.id.clone()).collect();
        assert!(trap_ids.contains(&blood_moon.id));
    }

    #[test]
    fn b2_7_the_view_names_the_type_only_where_it_differs_from_the_definition_s() {
        let mut state = game();
        put(&mut state, &blood_moon.id, slot(P1, Row::Backrow, 1), json!({}));
        put(&mut state, &blood_moon.id, slot(P1, Row::Backrow, 2), json!({ "radiant": true }));
        let own = view_for(&state, P1).you.backrow;
        assert!(matches_object(&to_json(&own[0]), &json!({ "faceDown": false, "type": "Trap" })));
        assert!(matches_object(&to_json(&own[1]), &json!({ "faceDown": false, "type": "Field Trap" })));
        let HandView::Cards(hand) = view_for(&state, P1).you.hand else {
            panic!("own hand is a list");
        };
        for card in &hand {
            assert_eq!(card.type_, None);
        }
        let held = in_hand(&mut state, &blood_moon.id, P1, 1);
        let Some(held) = held.first().cloned() else {
            panic!("no card");
        };
        find_instance_mut(&mut state, &held.id).expect("in hand").radiant = true;
        let HandView::Cards(after) = view_for(&state, P1).you.hand else {
            panic!("own hand is a list");
        };
        assert_eq!(
            after.iter().find(|card| card.instance_id == held.id).and_then(|card| card.type_),
            Some(CardType::FieldTrap)
        );
        // The other player reads the face-down cards as zones only (R351).
        let opponent = view_for(&state, P2).opponent.backrow;
        assert_eq!(
            to_json(&opponent[0..2].to_vec()),
            json!([{ "faceDown": true, "cost": 1 }, { "faceDown": true, "cost": 1 }])
        );
    }
}

/// TS's module `let nonce = 0`.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn act(state: &GameState, body: ActionInput) -> GameState {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let result = reduce(state, &body.with_nonce(format!("fc{nonce}")));
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

/// `describe("B2.7 X in the stats (Buff Billy)")`.
mod b2_7_x_in_the_stats_buff_billy {
    use super::*;

    #[test]
    fn b2_7_an_x_stat_unit_played_for_x_is_xstats_x_and_its_cry_s_upgrades_land_on_that_body() {
        let mut state = begin_game(&instance_game("billy", None)).state;
        let keep: Vec<String> = state.players[P1].hand.iter().map(|c| c.id.clone()).collect();
        state = act(
            &state,
            ActionInput {
                body: ActionBody::Mulligan { keep },
                player_id: P1,
            },
        );
        let keep: Vec<String> = state.players[P2].hand.iter().map(|c| c.id.clone()).collect();
        state = act(
            &state,
            ActionInput {
                body: ActionBody::Mulligan { keep },
                player_id: P2,
            },
        );
        let cards = in_hand(&mut state, &billy.id, P1, 1);
        let Some(card) = cards.first().cloned() else {
            panic!("no card");
        };
        state.players[P1].mana.current = 3;
        state.players[P1].mana.max = 3;
        let play = legal_actions(&state, P1).into_iter().find(|action| {
            matches!(action, ActionBody::Play { instance_id, x: Some(2), .. } if *instance_id == card.id)
        });
        let Some(play) = play else {
            panic!("expected a play for X = 2");
        };
        state = act(
            &state,
            ActionInput {
                body: play,
                player_id: P1,
            },
        );
        let unit = state.players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .find(|c| c.id == card.id)
            .cloned();
        let Some(unit) = unit else {
            panic!("expected Billy on the field");
        };
        assert_eq!(unit.x, Some(2));
        // 6/6 from the X, and two Upgrades from its Cry on top.
        let tuning_keys = match &unit.tuning {
            None => 0,
            Some(tuning) => to_json(tuning).as_object().map_or(0, |keys| keys.len()),
        };
        assert!(tuning_keys > 0);
        let view = unit_view(&state, &unit);
        assert!(view.attack + view.max_health >= 12);
    }

    #[test]
    fn b2_7_the_radiant_face_multiplies_the_same_x_by_its_own_numbers_7x_7x() {
        let mut state = instance_game("billy-radiant", None);
        let cards = in_hand(&mut state, &billy.id, P1, 1);
        let Some(card) = cards.first().cloned() else {
            panic!("no card");
        };
        {
            let held = find_instance_mut(&mut state, &card.id).expect("in hand");
            held.radiant = true;
            held.x = Some(2);
        }
        let view = unit_view(&state, &live(&state, &card.id));
        assert_eq!((view.attack, view.max_health), (14, 14));
    }

    #[test]
    fn b2_7_with_no_x_a_summon_outside_a_play_it_arrives_0_0_and_the_state_check_collects_it() {
        let mut state = instance_game("billy-summon", None);
        state.turn = 3;
        state.active = P1;
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let unit_id;
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let effect = summon(json_as(json!({ "defId": billy.id })));
            {
                let mut ctx = make_context(
                    sink.reborrow(),
                    None,
                    HookOptions {
                        controller: Some(P1),
                        ..Default::default()
                    },
                );
                (effect.apply)(&mut ctx);
            }
            let unit = sink.state.players[P1].units[0].as_ref().and_then(|pile| pile.first()).cloned();
            let Some(unit) = unit else {
                panic!("expected a summon");
            };
            let view = unit_view(sink.state, &unit);
            assert_eq!((view.attack, view.max_health), (0, 0));
            unit_id = unit.id.clone();
            state_check(&mut sink);
        }
        assert_eq!(live(&state, &unit_id).zone.z(), ZoneName::Graveyard);
    }
}
