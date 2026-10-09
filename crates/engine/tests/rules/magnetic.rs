//! Magnetic (R1086, ME-MAGNETIC): the play option onto one of your acting, non-Immutable Units.
//! A Magnetic card resolves in full on top of its host, then fuses into it with the host kept
//! (R653's reading of Hearthstone's Magnetic), separate from Stack.
//! Fixture defs and scripts are registered here, as in `effects_summon.rs`'s `game()`.

use jackioh_engine::layers::unit_view;
use jackioh_engine::play_choices::magnetic_host_at;
use jackioh_engine::testkit::*;

use super::fixtures::field::{act, act_result, flush};
use super::fixtures::harness::{events_of_type, in_hand, put, slot};

// ---------------------------------------------------------------------------
// Fixture cards.
// ---------------------------------------------------------------------------

/// TS `defOfKind`, numbered from 1900.
fn def_of_kind(name: &str, index: i32, overrides: Value) -> CardDef {
    let mut def = json!({
        "id": format!("mg-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (magnetic)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 2, "health": 2, "keywords": [], "text": name },
        "radiant": { "attack": 4, "health": 4, "keywords": [], "text": format!("{name} radiant") },
    });
    if let (Some(base), Some(overrides)) = (def.as_object_mut(), overrides.as_object()) {
        for (key, value) in overrides {
            base.insert(key.clone(), value.clone());
        }
    }
    json_as(def)
}

/// A plain 2/2 host.
fn host() -> CardDef {
    def_of_kind("host", 1901, json!({}))
}

/// A 1/1 Magnetic Unit whose Cry pings the enemy hero for 2, so "resolves in full" is observable.
fn mag() -> CardDef {
    def_of_kind(
        "mag",
        1902,
        json!({
            "base": {
                "attack": 1,
                "health": 1,
                "keywords": [{ "kind": "Magnetic" }],
                "text": "Magnetic. Cry: ping.",
            },
            "radiant": {
                "attack": 2,
                "health": 2,
                "keywords": [{ "kind": "Magnetic" }],
                "text": "Magnetic. Cry: ping.",
            },
        }),
    )
}

/// A 1/1 with both Stack and Magnetic.
fn both() -> CardDef {
    def_of_kind(
        "both",
        1903,
        json!({
            "base": {
                "attack": 1,
                "health": 1,
                "keywords": [{ "kind": "Stack" }, { "kind": "Magnetic" }],
                "text": "Stack, Magnetic",
            },
            "radiant": {
                "attack": 2,
                "health": 2,
                "keywords": [{ "kind": "Stack" }, { "kind": "Magnetic" }],
                "text": "Stack, Magnetic",
            },
        }),
    )
}

/// A 2/2 Immutable Unit: no host (R23).
fn immutable() -> CardDef {
    def_of_kind(
        "immutable",
        1904,
        json!({
            "base": {
                "attack": 2,
                "health": 2,
                "keywords": [{ "kind": "Immutable" }],
                "text": "Immutable",
            },
            "radiant": {
                "attack": 4,
                "health": 4,
                "keywords": [{ "kind": "Immutable" }],
                "text": "Immutable",
            },
        }),
    )
}

/// A 1/1 Magnetic Unit whose Cry destroys the chosen Unit: aimed at its own host, the host leaves
/// before the fusion.
fn assassin() -> CardDef {
    def_of_kind(
        "assassin",
        1905,
        json!({
            "base": {
                "attack": 1,
                "health": 1,
                "keywords": [{ "kind": "Magnetic" }],
                "text": "Magnetic. Cry: destroy.",
            },
            "radiant": {
                "attack": 2,
                "health": 2,
                "keywords": [{ "kind": "Magnetic" }],
                "text": "Magnetic. Cry: destroy.",
            },
        }),
    )
}

fn defs() -> Vec<CardDef> {
    vec![host(), mag(), both(), immutable(), assassin()]
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    let cry = |to: Value, amount: i32| Script {
        cry: Some(hook(move |_ctx| {
            vec![jackioh_engine::effects::damage(json_as(json!({
                "to": to,
                "amount": amount,
            })))]
        })),
        ..Script::default()
    };
    scripts.insert(
        mag().id,
        CardScripts {
            base: cry(json!({ "of": "enemyHero" }), 2),
            radiant: cry(json!({ "of": "enemyHero" }), 2),
        },
    );
    scripts.insert(
        assassin().id,
        CardScripts {
            base: cry(json!({ "of": "chosen" }), 99),
            radiant: cry(json!({ "of": "chosen" }), 99),
        },
    );
    scripts
}

/// Past both mulligans, in p1's main phase of turn 1, with this file's fixtures registered.
fn playing(seed: &str) -> GameState {
    let mut state = super::fixtures::field::playing(seed);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    registry.extend(scripts());
    register_scripts(registry);
    state
}

fn play_input(instance_id: &str, lane: i32, magnetic: bool, targets: Value) -> Value {
    let mut input = json!({
        "type": "play",
        "instanceId": instance_id,
        "zone": { "row": "units", "lane": lane },
        "playerId": "p1",
    });
    if magnetic {
        input["magnetic"] = json!(true);
    }
    if !targets.is_null() {
        input["targets"] = targets;
    }
    input
}

fn magnetic_actions(state: &GameState, instance_id: &str) -> Vec<Value> {
    legal_actions(state, PlayerId::P1)
        .iter()
        .filter_map(|action| {
            let value = serde_json::to_value(action).expect("an action serialises");
            (value.get("instanceId").and_then(|id| id.as_str()) == Some(instance_id)
                && value.get("magnetic").and_then(|flag| flag.as_bool()) == Some(true))
            .then(|| value.clone())
        })
        .collect()
}

fn fused_events(events: &[GameEvent]) -> Vec<GameEvent> {
    events_of_type(events, GameEventType::Fused)
}

// ---------------------------------------------------------------------------
// R1086.
// ---------------------------------------------------------------------------

#[test]
fn r1086_offered_onto_each_friendly_acting_unit() {
    let mut state = playing("magnetic-offered");
    put(
        &mut state,
        &host().id,
        slot(PlayerId::P1, Row::Units, 1),
        json!({}),
    );
    put(
        &mut state,
        &host().id,
        slot(PlayerId::P1, Row::Units, 2),
        json!({}),
    );
    let mag = in_hand(&mut state, &mag().id, PlayerId::P1, 1).remove(0);
    flush(&mut state, PlayerId::P1, 10);

    // Each friendly acting Unit is offered as a Magnetic host.
    let offered = magnetic_actions(&state, &mag.id);
    assert_eq!(offered.len(), 2);
    let lanes: Vec<i32> = offered
        .iter()
        .map(|action| {
            action
                .get("zone")
                .and_then(|zone| zone.get("lane"))
                .and_then(|lane| lane.as_i64())
                .unwrap_or(0) as i32
        })
        .collect();
    assert!(lanes.contains(&1) && lanes.contains(&2));
}

#[test]
fn r1086_r23_none_onto_immutable_enemy_or_empty_and_reduce_agrees() {
    let mut state = playing("magnetic-refused");
    put(
        &mut state,
        &immutable().id,
        slot(PlayerId::P1, Row::Units, 1),
        json!({}),
    );
    put(
        &mut state,
        &host().id,
        slot(PlayerId::P2, Row::Units, 1),
        json!({}),
    );
    let mag = in_hand(&mut state, &mag().id, PlayerId::P1, 1).remove(0);
    flush(&mut state, PlayerId::P1, 10);

    // The Immutable host, the enemy's Unit and the empty lane offer nothing Magnetic.
    assert!(magnetic_actions(&state, &mag.id).is_empty());
    // A Unit handed to the opponent in your own zone is no host either.
    put(
        &mut state,
        &host().id,
        slot(PlayerId::P1, Row::Units, 4),
        json!({}),
    );
    let handed = card_at(&state, slot(PlayerId::P1, Row::Units, 4))
        .expect("the unit")
        .clone();
    find_instance_mut(&mut state, &handed.id)
        .expect("the unit")
        .controller = PlayerId::P2;
    assert_eq!(
        magnetic_host_at(&state, PlayerId::P1, &slot(PlayerId::P1, Row::Units, 4), &[]),
        None
    );
    // And reduce agrees: the Magnetic play onto the Immutable host is refused.
    let result = act_result(&state, play_input(&mag.id, 1, true, Value::Null));
    assert!(result.error.is_some());
}

#[test]
fn r1086_resolves_on_top_then_fuses_host_kept() {
    let mut state = playing("magnetic-fuse");
    let host_card = in_hand(&mut state, &host().id, PlayerId::P1, 1).remove(0);
    let mag = in_hand(&mut state, &mag().id, PlayerId::P1, 1).remove(0);
    flush(&mut state, PlayerId::P1, 10);
    let hero_before = state.players.p2.hero.health;

    // The host is played, exerted, wounded and arrived on an earlier turn: everything the fusion
    // keeps is set apart from the values the Magnetic card brings, so the assertions read the keep.
    let mut state = act(&state, play_input(&host_card.id, 1, false, Value::Null));
    let host = find_instance_mut(&mut state, &host_card.id).expect("the host");
    host.attacked = true;
    host.damage = 1;
    host.summoned_turn = Some(7);

    let result = act_result(&state, play_input(&mag.id, 1, true, Value::Null));
    assert_eq!(result.error, None);
    let next = result.state;

    // The Cry ran in full on top: the enemy hero took the ping.
    assert_eq!(next.players.p2.hero.health, hero_before - 2);
    // Then the top card fused into the host, which the fusion kept.
    assert_eq!(fused_events(&result.events).len(), 1);
    let fused = card_at(&next, slot(PlayerId::P1, Row::Units, 1)).expect("the fused host");
    assert_eq!(fused.id, host_card.id);
    let view = unit_view(&next, fused);
    assert_eq!((view.attack, view.health), (3, 2));
    assert_eq!(fused.damage, 1);
    assert_eq!(fused.summoned_turn, Some(7));
    assert!(fused.attacked);
    // The top card is Gone with no Death: no `destroyed` names it.
    assert!(matches!(
        find_instance(&next, &mag.id).expect("the top card").zone,
        Zone::Gone { .. }
    ));
    assert!(
        events_of_type(&result.events, GameEventType::Destroyed)
            .iter()
            .all(
                |event| !matches!(event, GameEvent::Destroyed { instance_id, .. } if instance_id == &mag.id)
            )
    );
}

#[test]
fn r1086_nothing_fuses_when_the_host_left_first() {
    let mut state = playing("magnetic-host-left");
    let host_card = in_hand(&mut state, &host().id, PlayerId::P1, 1).remove(0);
    let assassin = in_hand(&mut state, &assassin().id, PlayerId::P1, 1).remove(0);
    flush(&mut state, PlayerId::P1, 10);
    let mut state = act(&state, play_input(&host_card.id, 1, false, Value::Null));

    // The Cry destroys the host it stands on: with the host gone, nothing fuses.
    let host_id = host_card.id.clone();
    let result = act_result(
        &state,
        play_input(
            &assassin.id,
            1,
            true,
            json!([{ "pick": "instance", "instanceId": host_id }]),
        ),
    );
    assert_eq!(result.error, None);
    let next = result.state;

    assert!(fused_events(&result.events).is_empty());
    assert!(
        events_of_type(&result.events, GameEventType::Destroyed)
            .iter()
            .any(
                |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if instance_id == &host_id)
            )
    );
    // The assassin stays the pile's top.
    assert_eq!(
        card_at(&next, slot(PlayerId::P1, Row::Units, 1)).map(|card| card.id.clone()),
        Some(assassin.id.clone())
    );
    assert!(matches!(
        find_instance(&next, &host_id).expect("the host").zone,
        Zone::Graveyard { .. }
    ));
}

#[test]
fn r1086_stack_and_magnetic_are_two_plays() {
    let mut state = playing("magnetic-stack");
    put(
        &mut state,
        &host().id,
        slot(PlayerId::P1, Row::Units, 1),
        json!({}),
    );
    let both = in_hand(&mut state, &both().id, PlayerId::P1, 1).remove(0);
    flush(&mut state, PlayerId::P1, 10);

    // The occupied zone is offered both ways: plain (Stack) and Magnetic.
    let actions = legal_actions(&state, PlayerId::P1);
    let plain = actions.iter().any(|action| {
        let value = serde_json::to_value(action).expect("an action serialises");
        value.get("instanceId").and_then(|id| id.as_str()) == Some(both.id.as_str())
            && value
                .get("zone")
                .and_then(|zone| zone.get("lane"))
                .and_then(|lane| lane.as_i64())
                == Some(1)
            && value.get("magnetic").and_then(|flag| flag.as_bool()).is_none()
    });
    assert!(plain);
    assert_eq!(magnetic_actions(&state, &both.id).len(), 1);

    // The Stack play lands on top and fuses nothing.
    let next = act(&state, play_input(&both.id, 1, false, Value::Null));
    assert_eq!(
        card_at(&next, slot(PlayerId::P1, Row::Units, 1)).map(|card| card.id.clone()),
        Some(both.id.clone())
    );
    assert!(find_instance(&next, &both.id).is_some());

    // The Magnetic play fuses instead.
    let mut state = playing("magnetic-stack-fuse");
    put(
        &mut state,
        &host().id,
        slot(PlayerId::P1, Row::Units, 1),
        json!({}),
    );
    let both = in_hand(&mut state, &both().id, PlayerId::P1, 1).remove(0);
    flush(&mut state, PlayerId::P1, 10);
    let result = act_result(&state, play_input(&both.id, 1, true, Value::Null));
    assert_eq!(result.error, None);
    assert_eq!(fused_events(&result.events).len(), 1);
}

#[test]
fn r1086_replays_to_the_same_hash() {
    let run = |seed: &str| -> String {
        let mut state = playing(seed);
        let host_card = in_hand(&mut state, &host().id, PlayerId::P1, 1).remove(0);
        let mag = in_hand(&mut state, &mag().id, PlayerId::P1, 1).remove(0);
        flush(&mut state, PlayerId::P1, 10);
        let state = act(&state, play_input(&host_card.id, 1, false, Value::Null));
        let done = act(&state, play_input(&mag.id, 1, true, Value::Null));
        hash_state(&done)
    };
    // The Magnetic fusion is deterministic: the same seed replays to the same hash.
    assert_eq!(run("magnetic-replay"), run("magnetic-replay"));
}
