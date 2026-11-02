//! Equip (R1086, ME-EQUIP): the play option onto one of your acting, non-Immutable Units.
//! An Equip card resolves in full on top of its host, then fuses into it with the host kept
//! (R653's reading of Hearthstone's Mech fusion), separate from Stack.

use jackioh_engine::layers::unit_view;
use jackioh_engine::play_choices::equip_host_at;
use jackioh_engine::testkit::*;

use super::fixtures::field::flush;
use super::fixtures::harness::{act, act_result, events_of_type, in_hand, put, slot};

// Fixture cards.

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

fn stack_and_equip() -> CardDef {
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

/// An enemy 2/2 that, at a play's announce (§10.5 step 3a), destroys the Unit in the zone it names: the
/// host leaves first. (The Equip card's own Cry cannot pick the dormant host, R13, R1086.)
fn sentry() -> CardDef {
    def_of_kind("sentry", 1905, json!({}))
}

/// Core #56 Jilliax's shape (R831013): a (1) 1/1 that is Equip at its embiggen price of (2).
fn paid() -> CardDef {
    def_of_kind("paid", 1906, json!({ "cost": { "base": 1, "embiggen": 2 } }))
}

/// Stinkyscape #16 Rod of Ages's shape (R1869000): a 2/2 that lends Equip, so any Unit card may equip it.
fn lender() -> CardDef {
    def_of_kind("lender", 1907, json!({}))
}

/// Stinkyscape #16 Rod of Ages's own shape (#2280): a statless Animated Field Spell that lends Equip,
/// so it hosts from its backrow zone (R1869000, R2280000).
fn rod_shape() -> CardDef {
    def_of_kind(
        "rod",
        1908,
        json!({
            "type": "Field Spell",
            "base": { "keywords": [{ "kind": "Animated" }], "text": "rod" },
            "radiant": { "keywords": [{ "kind": "Animated" }], "text": "rod radiant" },
        }),
    )
}

fn defs() -> Vec<CardDef> {
    vec![
        host(),
        mag(),
        stack_and_equip(),
        immutable(),
        sentry(),
        paid(),
        lender(),
        rod_shape(),
    ]
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
    let lends = || Script {
        static_flags: Some(json_as(json!({ "lendsEquip": true }))),
        ..Script::default()
    };
    scripts.insert(
        lender().id,
        CardScripts {
            base: lends(),
            radiant: lends(),
        },
    );
    scripts.insert(
        rod_shape().id,
        CardScripts {
            base: lends(),
            radiant: lends(),
        },
    );
    scripts
}

/// Past both mulligans, p1's main phase of turn 1, this file's fixtures registered.
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
            let value = serde_json::to_value(action).expect("an action serializes");
            (value.get("instanceId").and_then(|id| id.as_str()) == Some(instance_id)
                && value.get("equip").and_then(|flag| flag.as_bool()) == Some(true))
            .then(|| value.clone())
        })
        .collect()
}

fn fused_events(events: &[GameEvent]) -> Vec<GameEvent> {
    events_of_type(events, GameEventType::Fused)
}

// R1086.

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

    // The Immutable host, the enemy's Unit and the empty lane offer no Equip play.
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

    // Exerted, wounded and from an earlier turn, the host differs from what the Equip card brings.
    let mut state = act(&state, play_input(&host_card.id, 1, false, Value::Null));
    let host = find_instance_mut(&mut state, &host_card.id).expect("the host");
    host.exertion.attacked = true;
    host.damage = 1;
    host.summoned_turn = Some(7);

    let result = act_result(&state, play_input(&mag.id, 1, true, Value::Null));
    assert_eq!(result.error, None);
    let next = result.state;

    // The Cry ran in full on top: the ping landed.
    assert_eq!(next.players.p2.hero.health, hero_before - 2);
    // Then it fused into the host, which was kept.
    assert_eq!(fused_events(&result.events).len(), 1);
    let fused = card_at(&next, slot(PlayerId::P1, Row::Units, 1)).expect("the fused host");
    assert_eq!(fused.id, host_card.id);
    let view = unit_view(&next, fused);
    assert_eq!((view.attack, view.health), (3, 2));
    assert_eq!(fused.damage, 1);
    assert_eq!(fused.summoned_turn, Some(7));
    assert!(fused.exertion.attacked);
    // The top card ceased to exist with no Death: in no pile (R86), and named by no `destroyed`.
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

    // The sentry destroys the host on the announce: the card lands, its Cry still runs, nothing fuses.
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
    let both = in_hand(&mut state, &stack_and_equip().id, PlayerId::P1, 1).remove(0);
    flush(&mut state, PlayerId::P1, 10);

    // The occupied zone is offered both ways: plain (Stack) and Equip.
    let actions = legal_actions(&state, PlayerId::P1);
    let plain = actions.iter().any(|action| {
        let value = serde_json::to_value(action).expect("an action serializes");
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
    let both = in_hand(&mut state, &stack_and_equip().id, PlayerId::P1, 1).remove(0);
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
    assert_eq!(run("equip-replay"), run("equip-replay"));
}

// R781000: older snapshots and logs spell Equip and the play's flag as Magnetic; both read as Equip,
// written back under the new name only.
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

// R1869000 (Stinkyscape #16 Rod of Ages): a host that lends Equip takes any Unit card of its
// controller's as an Equip play while it is no fusion.
mod r1869000_a_host_that_lends_equip {
    use super::*;

    /// A lender in p1's unit zone 1, two plain Units in hand and the mana for both.
    fn lending() -> (GameState, String, String, String) {
        let mut state = playing("equip-lender");
        put(
            &mut state,
            &lender().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let lender_id = card_at(&state, slot(PlayerId::P1, Row::Units, 1))
            .expect("the lender")
            .id
            .clone();
        let mut plain = in_hand(&mut state, &host().id, PlayerId::P1, 2);
        flush(&mut state, PlayerId::P1, 10);
        let second = plain.remove(1).id;
        let first = plain.remove(0).id;
        (state, lender_id, first, second)
    }

    fn lanes_of(actions: &[Value]) -> Vec<i64> {
        actions
            .iter()
            .filter_map(|action| action["zone"]["lane"].as_i64())
            .collect()
    }

    #[test]
    fn r1869000_a_plain_unit_is_offered_onto_a_lender_and_fuses_into_it() {
        let (state, lender_id, first, _) = lending();
        assert_eq!(lanes_of(&equip_actions(&state, &first)), vec![1]);

        let result = act_result(&state, play_input(&first, 1, true, Value::Null));
        assert_eq!(result.error, None);
        assert_eq!(fused_events(&result.events).len(), 1);
        let fused = card_at(&result.state, slot(PlayerId::P1, Row::Units, 1)).expect("the fusion");
        assert_eq!(fused.id, lender_id, "the lender is the kept instance");
        let view = unit_view(&result.state, fused);
        assert_eq!((view.attack, view.health), (4, 4), "the two 2/2s summed");
        assert!(find_instance(&result.state, &first).is_none());
    }

    #[test]
    fn r1869000_a_lender_that_holds_a_fusion_lends_no_more() {
        let (state, _, first, second) = lending();
        let state = act(&state, play_input(&first, 1, true, Value::Null));

        assert!(equip_actions(&state, &second).is_empty());
        let refused = act_result(&state, play_input(&second, 1, true, Value::Null));
        assert!(
            refused
                .error
                .as_deref()
                .is_some_and(|error| error.contains("is not Equip")),
            "{:?}",
            refused.error
        );
        // It still plays as usual, into an open zone.
        let plain = act_result(&state, play_input(&second, 2, false, Value::Null));
        assert_eq!(plain.error, None);
        assert!(fused_events(&plain.events).is_empty());
    }

    #[test]
    fn r1869000_a_plain_unit_is_not_offered_onto_a_plain_host_and_reduce_agrees() {
        let mut state = playing("equip-no-lender");
        put(
            &mut state,
            &host().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let plain = in_hand(&mut state, &host().id, PlayerId::P1, 1).remove(0);
        flush(&mut state, PlayerId::P1, 10);

        assert!(equip_actions(&state, &plain.id).is_empty());
        let refused = act_result(&state, play_input(&plain.id, 1, true, Value::Null));
        assert!(
            refused
                .error
                .as_deref()
                .is_some_and(|error| error.contains("is not Equip")),
            "{:?}",
            refused.error
        );
    }

    #[test]
    fn r1869000_an_equip_card_still_goes_onto_a_lender_and_the_enemys_lender_lends_nothing() {
        let (mut state, _, first, _) = lending();
        put(
            &mut state,
            &lender().id,
            slot(PlayerId::P2, Row::Units, 2),
            json!({}),
        );
        let mag = in_hand(&mut state, &mag().id, PlayerId::P1, 1).remove(0);

        assert_eq!(lanes_of(&equip_actions(&state, &mag.id)), vec![1]);
        assert_eq!(lanes_of(&equip_actions(&state, &first)), vec![1]);
    }

    #[test]
    fn r1869000_r2280001_a_lender_in_its_backrow_zone_takes_a_unit_that_fuses_in_and_it_animates() {
        let mut state = playing("equip-rod-backrow");
        let rod = put(
            &mut state,
            &rod_shape().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        let plain = in_hand(&mut state, &host().id, PlayerId::P1, 1).remove(0);
        flush(&mut state, PlayerId::P1, 10);

        // With no stats it is no Unit, yet it hosts in its backrow zone: the listing names that zone.
        assert_eq!(
            equip_host_at(&state, PlayerId::P1, &slot(PlayerId::P1, Row::Backrow, 2), &[]),
            Some(rod.id.clone())
        );
        let actions = equip_actions(&state, &plain.id);
        assert_eq!(actions.len(), 1, "{actions:?}");
        assert_eq!(actions[0]["zone"], json!({ "row": "backrow", "lane": 2 }));

        let mut input = play_input(&plain.id, 2, true, Value::Null);
        input["zone"] = json!({ "row": "backrow", "lane": 2 });
        let result = act_result(&state, input);
        assert_eq!(result.error, None);
        assert_eq!(fused_events(&result.events).len(), 1);
        assert!(find_instance(&result.state, &plain.id).is_none());
        // The fusion gave it the Unit's stats beside its Animated: it animated at the state check.
        assert!(card_at(&result.state, slot(PlayerId::P1, Row::Backrow, 2)).is_none());
        let animated = card_at(&result.state, slot(PlayerId::P1, Row::Units, 2)).expect("the Rod, animated");
        assert_eq!(animated.id, rod.id);
        let view = unit_view(&result.state, animated);
        assert_eq!((view.attack, view.health), (2, 2));
        assert_eq!(events_of_type(&result.events, GameEventType::Animated).len(), 1);
    }
}

mod r2260000_a_dormant_card_hosts_no_equip_play {
    use super::*;

    #[test]
    fn r2260000_an_equip_card_is_offered_no_play_onto_a_dormant_card_and_reduce_agrees_until_it_awakens() {
        let mut state = playing("equip-dormant");
        let sleeper = put(
            &mut state,
            &host().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        // A Dormant card with no countdown, waiting for an effect to awaken it (R1839000).
        find_instance_mut(&mut state, &sleeper.id)
            .expect("the unit")
            .slumber = Some(Slumber { turns: None });
        put(
            &mut state,
            &host().id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        let mag = in_hand(&mut state, &mag().id, PlayerId::P1, 1).remove(0);
        flush(&mut state, PlayerId::P1, 10);

        assert_eq!(
            equip_host_at(&state, PlayerId::P1, &slot(PlayerId::P1, Row::Units, 1), &[]),
            None
        );
        let lanes: Vec<i64> = equip_actions(&state, &mag.id)
            .iter()
            .filter_map(|action| action["zone"]["lane"].as_i64())
            .collect();
        assert_eq!(lanes, vec![2], "only the awake host is offered");
        let result = act_result(&state, play_input(&mag.id, 1, true, Value::Null));
        assert!(
            result.error.is_some(),
            "an Equip play onto the Dormant card is refused"
        );

        // Awake, it hosts as any Unit does.
        find_instance_mut(&mut state, &sleeper.id)
            .expect("the unit")
            .slumber = None;
        assert_eq!(
            equip_host_at(&state, PlayerId::P1, &slot(PlayerId::P1, Row::Units, 1), &[]),
            Some(sleeper.id.clone())
        );
    }
}
