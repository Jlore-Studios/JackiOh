//! Equip (R1086, ME-EQUIP): the play option onto one of your acting, non-Immutable Units.
//! An Equip card resolves in full on top of its host, then fuses into it with the host kept
//! (R653's reading of Hearthstone's Mech fusion), separate from Stack.
//! Fixture defs and scripts are registered here, as in `effects_summon.rs`'s `game()`.

use jackioh_engine::layers::unit_view;
use jackioh_engine::play_choices::equip_host_at;
use jackioh_engine::testkit::*;

use super::fixtures::field::{act, act_result, flush};
use super::fixtures::harness::{events_of_type, in_hand, put, slot};

// ---------------------------------------------------------------------------
// Fixture cards.
// ---------------------------------------------------------------------------

/// TS `defOfKind`, numbered from 1900.
fn def_of_kind(name: &str, index: i32, overrides: Value) -> CardDef {
    let mut def = json!({
        "id": format!("eq-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (equip)"),
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

/// A 1/1 Equip Unit whose Cry pings the enemy hero for 2, so "resolves in full" is observable.
fn mag() -> CardDef {
    def_of_kind(
        "mag",
        1902,
        json!({
            "base": {
                "attack": 1,
                "health": 1,
                "keywords": [{ "kind": "Equip" }],
                "text": "Equip. Cry: ping.",
            },
            "radiant": {
                "attack": 2,
                "health": 2,
                "keywords": [{ "kind": "Equip" }],
                "text": "Equip. Cry: ping.",
            },
        }),
    )
}

/// A 1/1 with both Stack and Equip.
fn both() -> CardDef {
    def_of_kind(
        "both",
        1903,
        json!({
            "base": {
                "attack": 1,
                "health": 1,
                "keywords": [{ "kind": "Stack" }, { "kind": "Equip" }],
                "text": "Stack, Equip",
            },
            "radiant": {
                "attack": 2,
                "health": 2,
                "keywords": [{ "kind": "Stack" }, { "kind": "Equip" }],
                "text": "Stack, Equip",
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

/// An enemy 2/2 that answers a play's announce (§10.5 step 3a) by destroying the Unit in the zone
/// the play named: aimed at an Equip play, its host leaves before the card is placed, let alone
/// fused. (A Cry of the Equip card cannot reach the host: the host lies dormant beneath it while
/// the Cry runs, R13, R1086, so a pick of it fizzles.)
fn sentry() -> CardDef {
    def_of_kind("sentry", 1905, json!({}))
}

/// Core #56 Jilliax's shape (R831013): a (1) 1/1 with an embiggen price of (2), at which it is Equip —
/// its `conditionalKeywords` hands Equip to a card stamped embiggened.
fn paid() -> CardDef {
    def_of_kind("paid", 1906, json!({ "cost": { "base": 1, "embiggen": 2 } }))
}

fn defs() -> Vec<CardDef> {
    vec![host(), mag(), both(), immutable(), sentry(), paid()]
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
    let watch = || Script {
        triggers: vec![
            TriggerDef::new("eq-sentry", &[GameEventType::CardAnnounced], |ctx, event| {
                let GameEvent::CardAnnounced {
                    player,
                    row: Some(row),
                    lane: Some(lane),
                    ..
                } = event
                else {
                    return vec![];
                };
                let Some(host) = card_at(ctx.state, slot(*player, *row, *lane)) else {
                    return vec![];
                };
                vec![jackioh_engine::effects::destroy(json_as(json!({
                    "target": { "of": "instance", "instanceId": host.id },
                })))]
            })
            .with_when(|ctx, event| {
                matches!(event, GameEvent::CardAnnounced { player, .. } if *player != ctx.controller)
            }),
        ],
        ..Script::default()
    };
    scripts.insert(
        sentry().id,
        CardScripts {
            base: watch(),
            radiant: watch(),
        },
    );
    let paid_equip = || Script {
        conditional_keywords: Some(read_hook(|args| {
            if args.self_.embiggened == Some(true) {
                vec![Keyword::Equip]
            } else {
                vec![]
            }
        })),
        ..Script::default()
    };
    scripts.insert(
        paid().id,
        CardScripts {
            base: paid_equip(),
            radiant: paid_equip(),
        },
    );
    scripts
}

/// Past both mulligans, in p1's main phase of turn 1, with this file's fixtures registered.
fn playing(seed: &str) -> GameState {
    let state = super::fixtures::field::playing(seed);
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

fn play_input(instance_id: &str, lane: i32, equip: bool, targets: Value) -> Value {
    let mut input = json!({
        "type": "play",
        "instanceId": instance_id,
        "zone": { "row": "units", "lane": lane },
        "playerId": "p1",
    });
    if equip {
        input["equip"] = json!(true);
    }
    if !targets.is_null() {
        input["targets"] = targets;
    }
    input
}

fn equip_actions(state: &GameState, instance_id: &str) -> Vec<Value> {
    legal_actions(state, PlayerId::P1)
        .iter()
        .filter_map(|action| {
            let value = serde_json::to_value(action).expect("an action serialises");
            (value.get("instanceId").and_then(|id| id.as_str()) == Some(instance_id)
                && value.get("equip").and_then(|flag| flag.as_bool()) == Some(true))
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
    let mut state = playing("equip-offered");
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

    // Each friendly acting Unit is offered as an Equip host.
    let offered = equip_actions(&state, &mag.id);
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
    let mut state = playing("equip-refused");
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

    // The Immutable host, the enemy's Unit and the empty lane offer nothing Equip.
    assert!(equip_actions(&state, &mag.id).is_empty());
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
        equip_host_at(&state, PlayerId::P1, &slot(PlayerId::P1, Row::Units, 4), &[]),
        None
    );
    // And reduce agrees: the Equip play onto the Immutable host is refused.
    let result = act_result(&state, play_input(&mag.id, 1, true, Value::Null));
    assert!(result.error.is_some());
}

#[test]
fn r1086_resolves_on_top_then_fuses_host_kept() {
    let mut state = playing("equip-fuse");
    let host_card = in_hand(&mut state, &host().id, PlayerId::P1, 1).remove(0);
    let mag = in_hand(&mut state, &mag().id, PlayerId::P1, 1).remove(0);
    flush(&mut state, PlayerId::P1, 10);
    let hero_before = state.players.p2.hero.health;

    // The host is played, exerted, wounded and arrived on an earlier turn: everything the fusion
    // keeps is set apart from the values the Equip card brings, so the assertions read the keep.
    let mut state = act(&state, play_input(&host_card.id, 1, false, Value::Null));
    let host = find_instance_mut(&mut state, &host_card.id).expect("the host");
    host.exertion.attacked = true;
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
    assert!(fused.exertion.attacked);
    // The top card ceased to exist with no Death: it is in no pile (R86), and no `destroyed`
    // names it.
    assert!(find_instance(&next, &mag.id).is_none());
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
    let mut state = playing("equip-host-left");
    let host_card = in_hand(&mut state, &host().id, PlayerId::P1, 1).remove(0);
    let mag = in_hand(&mut state, &mag().id, PlayerId::P1, 1).remove(0);
    flush(&mut state, PlayerId::P1, 10);
    let mut state = act(&state, play_input(&host_card.id, 1, false, Value::Null));
    put(
        &mut state,
        &sentry().id,
        slot(PlayerId::P2, Row::Units, 1),
        json!({}),
    );
    let hero_before = state.players.p2.hero.health;

    // The sentry destroys the host as the Equip play is announced: with the host gone, the card
    // lands in the emptied zone, its Cry still runs, and nothing fuses.
    let host_id = host_card.id.clone();
    let result = act_result(&state, play_input(&mag.id, 1, true, Value::Null));
    assert_eq!(result.error, None);
    let next = result.state;

    assert_eq!(next.players.p2.hero.health, hero_before - 2);
    assert!(fused_events(&result.events).is_empty());
    assert!(
        events_of_type(&result.events, GameEventType::Destroyed)
            .iter()
            .any(
                |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if instance_id == &host_id)
            )
    );
    // The Equip card stays the pile's top.
    assert_eq!(
        card_at(&next, slot(PlayerId::P1, Row::Units, 1)).map(|card| card.id.clone()),
        Some(mag.id.clone())
    );
    assert!(matches!(
        find_instance(&next, &host_id).expect("the host").zone,
        Zone::Graveyard { .. }
    ));
}

#[test]
fn r1086_stack_and_equip_are_two_plays() {
    let mut state = playing("equip-stack");
    put(
        &mut state,
        &host().id,
        slot(PlayerId::P1, Row::Units, 1),
        json!({}),
    );
    let both = in_hand(&mut state, &both().id, PlayerId::P1, 1).remove(0);
    flush(&mut state, PlayerId::P1, 10);

    // The occupied zone is offered both ways: plain (Stack) and Equip.
    let actions = legal_actions(&state, PlayerId::P1);
    let plain = actions.iter().any(|action| {
        let value = serde_json::to_value(action).expect("an action serialises");
        value.get("instanceId").and_then(|id| id.as_str()) == Some(both.id.as_str())
            && value
                .get("zone")
                .and_then(|zone| zone.get("lane"))
                .and_then(|lane| lane.as_i64())
                == Some(1)
            && value.get("equip").and_then(|flag| flag.as_bool()).is_none()
    });
    assert!(plain);
    assert_eq!(equip_actions(&state, &both.id).len(), 1);

    // The Stack play lands on top and fuses nothing.
    let next = act(&state, play_input(&both.id, 1, false, Value::Null));
    assert_eq!(
        card_at(&next, slot(PlayerId::P1, Row::Units, 1)).map(|card| card.id.clone()),
        Some(both.id.clone())
    );
    assert!(find_instance(&next, &both.id).is_some());

    // The Equip play fuses instead.
    let mut state = playing("equip-stack-fuse");
    put(
        &mut state,
        &host().id,
        slot(PlayerId::P1, Row::Units, 1),
        json!({}),
    );
    let both = in_hand(&mut state, &self::both().id, PlayerId::P1, 1).remove(0);
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
    // The Equip fusion is deterministic: the same seed replays to the same hash.
    assert_eq!(run("equip-replay"), run("equip-replay"));
}

// R781000: Magnetic is now Equip. The shipped snapshots (v0.3.5, v0.3.6) and a log recorded before the
// rename spell the keyword and the play's flag the old way, and both read as Equip, written back under
// the new name only.
mod r781000_the_old_name_reads_as_equip {
    use super::*;

    #[test]
    fn r781000_the_keyword_and_its_kind_read_magnetic_as_equip_and_write_equip() {
        let keyword: Keyword =
            serde_json::from_value(json!({ "kind": "Magnetic" })).expect("the old kind reads");
        assert_eq!(keyword, Keyword::Equip);
        assert_eq!(
            serde_json::to_value(&keyword).expect("writes"),
            json!({ "kind": "Equip" })
        );
        let kind: KeywordKind = serde_json::from_value(json!("Magnetic")).expect("the old kind reads");
        assert_eq!(kind, KeywordKind::Equip);
        assert_eq!(kind.as_str(), "Equip");
    }

    #[test]
    fn r781000_the_play_actions_old_flag_reads_as_equip_and_writes_equip() {
        let body: ActionBody = serde_json::from_value(json!({
            "type": "play",
            "instanceId": "c7",
            "zone": { "row": "units", "lane": 1 },
            "magnetic": true,
        }))
        .expect("the old flag reads");
        assert!(
            matches!(
                body,
                ActionBody::Play {
                    equip: Some(true),
                    ..
                }
            ),
            "{body:?}"
        );
        let written = serde_json::to_value(&body).expect("writes");
        assert_eq!(written["equip"], json!(true));
        assert!(written.get("magnetic").is_none());
    }
}

mod r831013_an_equip_a_price_grants {
    use super::*;

    #[test]
    fn r831013_is_offered_only_at_that_price_and_reduce_agrees() {
        let mut state = playing("equip-paid");
        put(
            &mut state,
            &host().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        state.players.p1.mana.current = 5;
        let card = in_hand(&mut state, &paid().id, PlayerId::P1, 1).remove(0);
        // Every Equip play listed is at the embiggen price; the base price lists plain plays only.
        let offered = equip_actions(&state, &card.id);
        assert!(!offered.is_empty(), "an Equip play onto the host at (2)");
        assert!(offered.iter().all(|action| action["embiggen"] == json!(true)));
        let mut base = play_input(&card.id, 1, true, Value::Null);
        base["embiggen"] = json!(false);
        assert!(act_result(&state, base).error.is_some(), "no Equip at (1)");
        let mut embiggened = play_input(&card.id, 1, true, Value::Null);
        embiggened["embiggen"] = json!(true);
        let after = act_result(&state, embiggened);
        assert!(after.error.is_none(), "{:?}", after.error);
        assert_eq!(fused_events(&after.events).len(), 1, "it fused into the host");
        assert_eq!(after.state.players.p1.mana.current, 5 - 2);
    }
}
