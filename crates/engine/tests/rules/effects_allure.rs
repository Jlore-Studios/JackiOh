//! ME-ALLURE (Meditative #39.5 Jade Beauty; docs/meditative-set.md M5, Group C's systems): "Allure
//! every enemy Unit" marks each one `allure` (pink, R437); at the Allurer's next start each marked Unit
//! still the opponent's is stolen in lane order (R15), summoning sick (R171), or, with no open zone,
//! destroyed as any destroy is: Death, Reborn and Indestructible apply (R46, R963, MD-C4; fixtures/jade.rs).

use jackioh_engine::effects::delay::DELAYED_ALLURE_HOOK;
use jackioh_engine::effects::destroy::{DestroyArgs, destroy};
use jackioh_engine::effects::steal::StealOtherwise;
use jackioh_engine::effects::targets::TargetSpec;
use jackioh_engine::reduce::begin_game;
use jackioh_engine::state_check::state_check;
use jackioh_engine::testkit::*;
use jackioh_engine::view_for::view_for;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::catalog::vanilla_catalog;
use crate::rules::fixtures::harness::{act_ok, events_of_type, in_hand, json_of, new_game, put, slot};
use crate::rules::fixtures::jade::{JADE_SCRIPTS, jade_allure, jade_catalog};
use crate::rules::fixtures::scripts::{FIXTURE_SCRIPTS, fixture_catalog};

fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(&format!("allure-{seed}"), None)).state;
    register_catalog(jade_catalog(fixture_catalog(vanilla_catalog(2 * DECK_SIZE, 1))));
    let mut scripts = FIXTURE_SCRIPTS.clone();
    scripts.extend(JADE_SCRIPTS.clone());
    register_scripts(scripts);
    for player in [P1, P2] {
        let keep: Vec<String> = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        state = act_ok(
            &state,
            json!({ "type": "mulligan", "keep": keep, "playerId": player }),
        )
        .state;
    }
    state.players.p1.auto_end_turn = Some(false);
    state.players.p2.auto_end_turn = Some(false);
    state
}

/// Put `def_id` in `player`'s hand and play it, `extra` merged into the action.
fn play(state: &mut GameState, player: PlayerId, def_id: &str, extra: Value) -> ReduceResult {
    let card = in_hand(state, def_id, player, 1)
        .into_iter()
        .next()
        .expect("a card to play");
    let mut body = json!({ "type": "play", "instanceId": card.id, "playerId": player });
    if let (Some(body), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            body.insert(key.clone(), value.clone());
        }
    }
    act_ok(state, body)
}

fn allure(state: &mut GameState, player: PlayerId) -> ReduceResult {
    play(state, player, &jade_allure.id, json!({}))
}

fn pass(state: &GameState) -> ReduceResult {
    act_ok(state, json!({ "type": "endTurn", "playerId": state.active }))
}

fn top_id(state: &GameState, player: PlayerId, lane: i32) -> Option<String> {
    state.players[player]
        .units
        .get((lane - 1) as usize)?
        .as_ref()?
        .first()
        .map(|card| card.id.clone())
}

fn marks_of(view: &PlayerView, player: PlayerId, lane: i32) -> Vec<(String, String)> {
    let side = if view.viewer == player {
        &view.you
    } else {
        &view.opponent
    };
    side.units
        .get((lane - 1) as usize)
        .and_then(|unit| unit.as_ref())
        .and_then(|unit| unit.marks.clone())
        .unwrap_or_default()
        .into_iter()
        .map(|mark| (mark.mark, mark.color))
        .collect()
}

#[test]
fn r963_marks_every_enemy_unit_allure_pink_in_both_views() {
    let mut state = playing("marks");
    let first = put(&mut state, "fx-1", slot(P2, Row::Units, 1), json!({}));
    let second = put(&mut state, "fx-2", slot(P2, Row::Units, 3), json!({}));
    let outcome = allure(&mut state, P1);
    let cast = outcome.state;
    for viewer in [P1, P2] {
        let view = view_for(&cast, viewer);
        assert_eq!(
            marks_of(&view, P2, 1),
            vec![("allure".to_string(), "pink".to_string())]
        );
        assert_eq!(
            marks_of(&view, P2, 3),
            vec![("allure".to_string(), "pink".to_string())]
        );
        assert!(marks_of(&view, P2, 2).is_empty());
    }
    let marked: Vec<Value> = events_of_type(&outcome.events, GameEventType::Marked)
        .into_iter()
        .map(json_of)
        .collect();
    assert!(
        marked
            .iter()
            .any(|event| event["instanceId"] == json!(first.id) && event["added"] == json!(true))
    );
    assert!(
        marked
            .iter()
            .any(|event| event["instanceId"] == json!(second.id) && event["added"] == json!(true))
    );
}

#[test]
fn r963_steals_each_marked_unit_in_lane_order_at_the_allurers_next_start_summoning_sick() {
    let mut state = playing("steal-order");
    let far = put(&mut state, "fx-1", slot(P2, Row::Units, 3), json!({}));
    let near = put(&mut state, "fx-2", slot(P2, Row::Units, 1), json!({}));
    let cast = allure(&mut state, P1).state;
    assert_eq!(cast.active, P1);
    // The opponent's turn passes the Allure by; the Allurer's next start lands it.
    let theirs = pass(&cast).state;
    assert_eq!(theirs.active, P2);
    assert_eq!(
        find_instance(&theirs, &near.id).map(|card| card.controller),
        Some(P2)
    );
    let ReduceResult {
        state: mine, events, ..
    } = pass(&theirs);
    assert_eq!(mine.active, P1);
    for id in [&near.id, &far.id] {
        let card = find_instance(&mine, id).expect("the stolen Unit");
        assert_eq!(card.controller, P1);
        // R171: it entered its new side this turn, so it is summoning sick there.
        assert_eq!(card.summoned_turn, Some(mine.turn));
    }
    let stolen: Vec<Value> = events_of_type(&events, GameEventType::ControlChanged)
        .into_iter()
        .map(json_of)
        .collect();
    assert_eq!(
        stolen
            .iter()
            .map(|event| event["instanceId"].clone())
            .collect::<Vec<_>>(),
        vec![json!(near.id), json!(far.id)],
        "lane order"
    );
}

#[test]
fn r963_no_open_zone_destroys_it_and_an_indestructible_one_stays() {
    let mut state = playing("no-room");
    for lane in 1..=5 {
        put(&mut state, "fx-10", slot(P1, Row::Units, lane), json!({}));
    }
    let victim = put(&mut state, "fx-1", slot(P2, Row::Units, 1), json!({}));
    let tough = put(&mut state, "fx-2", slot(P2, Row::Units, 2), json!({}));
    find_instance_mut(&mut state, &tough.id)
        .expect("the tough Unit")
        .granted_keywords = vec![json_as(json!({ "kind": "Indestructible" }))];
    let cast = allure(&mut state, P1).state;
    let ReduceResult {
        state: mine, events, ..
    } = pass(&pass(&cast).state);
    assert_eq!(mine.active, P1);
    // No open zone: the ordinary Unit is destroyed, so its Death side applies.
    assert!(find_instance(&mine, &victim.id).is_none_or(|card| card.zone.z() != ZoneName::Field));
    assert!(
        events_of_type(&events, GameEventType::Destroyed)
            .iter()
            .any(|event| json_of(event)["instanceId"] == json!(victim.id)),
        "the full row destroys it"
    );
    // R46: an Indestructible Unit stays where it is.
    let kept = find_instance(&mine, &tough.id).expect("the Indestructible Unit");
    assert_eq!(kept.controller, P2);
    assert_eq!(top_id(&mine, P2, 2), Some(tough.id));
}

#[test]
fn r963_a_marked_unit_that_left_the_field_is_not_taken() {
    let mut state = playing("left");
    let victim = put(&mut state, "fx-1", slot(P2, Row::Units, 1), json!({}));
    allure(&mut state, P1);
    // Destroyed meanwhile through the real path (R174): §4.5's state check collects the mark and
    // drops the waiting entry with the stay, so the next start takes nothing.
    let spell = {
        let mut card = new_instance(&mut state, "fx-mana-well", P2, Zone::Resolving { player: P2 });
        card.radiant = false;
        card
    };
    {
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                Some(&spell),
                HookOptions {
                    controller: Some(P2),
                    targets: Some(vec![]),
                    ..Default::default()
                },
            );
            (destroy(DestroyArgs {
                target: TargetSpec::Instance {
                    instance_id: victim.id.clone(),
                },
            })
            .apply)(&mut ctx);
            state_check(&mut sink);
        }
        state.rng_cursor = rng.cursor();
    }
    let ReduceResult {
        state: mine, events, ..
    } = pass(&pass(&state).state);
    assert_eq!(mine.active, P1);
    let card = find_instance(&mine, &victim.id).expect("the destroyed Unit");
    assert_eq!(card.controller, P2);
    assert_eq!(card.zone.z(), ZoneName::Graveyard);
    assert!(
        events_of_type(&events, GameEventType::ControlChanged).is_empty(),
        "nothing is stolen"
    );
}

#[test]
fn r963_it_lands_after_the_allurer_left() {
    let mut state = playing("allurer-gone");
    let victim = put(&mut state, "fx-1", slot(P2, Row::Units, 1), json!({}));
    let cast = allure(&mut state, P1).state;
    // R76: the Allure is a Spell, gone to the graveyard with its resolution — yet its entries wait.
    assert!(
        cast.players[P1]
            .graveyard
            .iter()
            .any(|card| card.def_id == jade_allure.id),
        "the Allure left the field"
    );
    assert!(
        cast.delayed
            .iter()
            .any(|entry| entry.resume.hook == DELAYED_ALLURE_HOOK),
        "its entries wait"
    );
    let mine = pass(&pass(&cast).state).state;
    assert_eq!(
        find_instance(&mine, &victim.id).map(|card| card.controller),
        Some(P1)
    );
}

#[test]
fn r963_one_already_yours_is_neither_stolen_nor_destroyed() {
    let mut state = playing("own");
    for lane in 1..=4 {
        put(&mut state, "fx-10", slot(P1, Row::Units, lane), json!({}));
    }
    let own = put(&mut state, "fx-11", slot(P1, Row::Units, 5), json!({}));
    let victim = put(&mut state, "fx-1", slot(P2, Row::Units, 1), json!({}));
    let cast = allure(&mut state, P1).state;
    // Only the enemy Unit is marked; the row is full, so the steal becomes a destroy of it alone.
    let ReduceResult {
        state: mine, events, ..
    } = pass(&pass(&cast).state);
    let kept = find_instance(&mine, &own.id).expect("your own Unit");
    assert_eq!(kept.controller, P1);
    assert_eq!(top_id(&mine, P1, 5), Some(own.id.clone()));
    assert!(
        events_of_type(&events, GameEventType::Destroyed)
            .iter()
            .all(|event| json_of(event)["instanceId"] != json!(own.id)),
        "your own Unit is never destroyed"
    );
    assert!(find_instance(&mine, &victim.id).is_none_or(|card| card.zone.z() != ZoneName::Field));
}

/// One `steal` by id at `victim_id`, run as P1 into a full row, so it cannot land; its events.
fn run_full_row_steal(
    state: &mut GameState,
    victim_id: &str,
    otherwise: Option<StealOtherwise>,
) -> Vec<GameEvent> {
    use jackioh_engine::effects::steal::{StealTarget, steal};
    let spell = {
        let mut card = new_instance(state, "fx-mana-well", P1, Zone::Resolving { player: P1 });
        card.radiant = false;
        card
    };
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            Some(&spell),
            HookOptions {
                controller: Some(P1),
                targets: Some(vec![]),
                ..Default::default()
            },
        );
        (steal(StealTarget {
            instance_id: Some(victim_id.to_string()),
            otherwise,
            ..StealTarget::default()
        })
        .apply)(&mut ctx);
        // `steal` only marks for `otherwise: destroy` (§6.3 Destroy is a mark; §4.5's state check
        // collects it), so settle before reading the events.
        state_check(&mut sink);
    }
    state.rng_cursor = rng.cursor();
    events
}

#[test]
fn r963_steal_otherwise_destroy_needs_no_room_to_destroy() {
    let mut state = playing("otherwise-direct");
    for lane in 1..=5 {
        put(&mut state, "fx-10", slot(P1, Row::Units, lane), json!({}));
    }
    let victim = put(&mut state, "fx-1", slot(P2, Row::Units, 1), json!({}));
    let events = run_full_row_steal(&mut state, &victim.id, Some(StealOtherwise::Destroy));
    assert!(
        events.iter().any(|event| matches!(
            event,
            GameEvent::Destroyed { instance_id, .. } if instance_id == &victim.id
        )),
        "a full row still destroys it"
    );
    // Without `otherwise` the same steal stays a refusal: the Unit stands.
    let mut state = playing("otherwise-plain");
    for lane in 1..=5 {
        put(&mut state, "fx-10", slot(P1, Row::Units, lane), json!({}));
    }
    let victim = put(&mut state, "fx-1", slot(P2, Row::Units, 1), json!({}));
    let events = run_full_row_steal(&mut state, &victim.id, None);
    assert!(events.is_empty(), "a plain steal refused reports nothing");
    assert_eq!(top_id(&state, P2, 1), Some(victim.id));
}

/// Apply `effect` for `controller` with no card running it, then settle.
fn apply_settled(state: &mut GameState, controller: PlayerId, effect: Effect) -> Vec<GameEvent> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        {
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(controller),
                    ..Default::default()
                },
            );
            (effect.apply)(&mut ctx);
        }
        settle(&mut sink, SettleOptions::default());
    }
    state.rng_cursor = rng.cursor();
    events
}

#[test]
fn r1876002_remove_allures_withdraws_every_allure_on_the_field_so_nothing_is_stolen() {
    let mut state = playing("remove");
    let theirs = put(&mut state, "fx-1", slot(P2, Row::Units, 1), json!({}));
    let mut cast = allure(&mut state, P1).state;
    assert!(
        jackioh_engine::marks::marks_on(&cast, &theirs.id)
            .iter()
            .any(|mark| mark.mark == "allure")
    );
    let events = apply_settled(&mut cast, P2, jackioh_engine::effects::delay::remove_allures());
    // The mark comes off, said by a `marked` event, and no Allure waits.
    assert!(jackioh_engine::marks::marks_on(&cast, &theirs.id).is_empty());
    assert!(events.iter().any(|event| matches!(
        event,
        GameEvent::Marked { instance_id, added: false, .. } if *instance_id == theirs.id
    )));
    assert!(
        cast.delayed
            .iter()
            .all(|entry| entry.resume.hook != DELAYED_ALLURE_HOOK)
    );
    let mine = pass(&pass(&cast).state).state;
    assert_eq!(mine.active, P1);
    assert_eq!(
        find_instance(&mine, &theirs.id).map(|card| card.controller),
        Some(P2),
        "not stolen"
    );
}
