//! What Armor takes off a hit, reported (patch v0.3.X, docs/meditative-set.md M8, MN05; SPEC §4.4,
//! §6.1's Armor and Pierce rows, §10.3, §10.8).
//!
//! R1360: the `damage` event carries the Armor's part of the hit, `absorbed`: a unit's Armor or a
//! hero's at step 2, never a hero's divisors or cap, 0 under Pierce, the Armor of the hero a
//! redirected hit lands on, and a Trample excess's own. It is left off the wire at 0 (D14).
//!
//! R1361: a hit the Armor takes whole is reported by `damageAbsorbed`, and R63's zero rule still holds
//! for every rule that reads damage: no `damage`, no on-damage trigger, no trap, no kill credit, no
//! Poisonous, no Lifesteal and no Trample excess come of it (the quest half is in `quests.rs`, the
//! fatigue half, R1362, in the cards' `turn_stages.rs`).

use jackioh_engine::effects::{heal, plague};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::combat::{
    armoured, indestructible, lifestealer, plain, poisonous, shielded, trampler,
};
use crate::rules::fixtures::damage_combat::{argus, gambit, playing};
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, slot};
use crate::rules::fixtures::scripts::anti_oneshot;

fn game(seed: &str) -> GameState {
    new_game(seed, None)
}

/// The card as the state holds it now.
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id)
        .cloned()
        .unwrap_or_else(|| panic!("{id} is in no zone"))
}

/// A fresh sink over `state`, its rng at the state's cursor as `reduce` builds it.
fn with_sink<R>(state: &mut GameState, run: impl FnOnce(&mut EngineSink) -> R) -> R {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    run(&mut sink)
}

enum Aim<'a> {
    Unit(&'a str),
    Hero(PlayerId),
}

/// One damage instance on the sink, each card read as it stands now; the amount it dealt.
fn hit(sink: &mut EngineSink, source: Option<&str>, aim: Aim<'_>, amount: i32, flags: Option<Value>) -> i32 {
    let source = source.map(|id| live(sink.state, id));
    let target = match aim {
        Aim::Unit(id) => DamageTarget::Unit {
            instance: live(sink.state, id),
        },
        Aim::Hero(player) => DamageTarget::Hero { player },
    };
    deal_damage(
        sink,
        DamageArgs {
            source,
            target,
            amount,
            flags: flags.map(json_as),
        },
    )
}

/// One hit on a sink of its own: what it dealt, and every event it emitted, as JSON.
fn hit_alone(
    state: &mut GameState,
    source: Option<&str>,
    aim: Aim<'_>,
    amount: i32,
    flags: Option<Value>,
) -> (i32, Vec<Value>) {
    with_sink(state, |sink| {
        let dealt = hit(sink, source, aim, amount, flags);
        (dealt, wire(sink.events))
    })
}

fn wire(events: &[GameEvent]) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .collect()
}

fn grant(state: &mut GameState, id: &str, keyword: Keyword) {
    find_instance_mut(state, id)
        .expect("on the field")
        .granted_keywords
        .push(keyword);
}

mod r1360_the_damage_event_carries_the_armor_s_part {
    use super::*;

    #[test]
    fn r1360_a_partial_hit_carries_what_the_armor_took_on_a_unit_and_on_a_hero_and_none_where_it_took_nothing()
     {
        let mut state = game("r1360-partial");
        let source = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let wall = put(&mut state, &armoured.id, slot(P2, Row::Units, 1), json!({})); // Armor 7

        let (dealt, events) = hit_alone(
            &mut state,
            Some(&source.id),
            Aim::Unit(&wall.id),
            10,
            Some(json!({ "combat": true })),
        );
        assert_eq!(dealt, 3);
        assert_eq!(
            events,
            vec![
                json!({ "type": "damage", "sourceId": source.id, "targetId": wall.id, "amount": 3, "combat": true, "absorbed": 7 })
            ]
        );

        // A hero's Armor takes its part the same way (§4.4 step 2, R124).
        state.players.p2.hero.armor = 2;
        let (dealt, events) = hit_alone(&mut state, None, Aim::Hero(P2), 5, None);
        assert_eq!(dealt, 3);
        assert_eq!(
            events,
            vec![
                json!({ "type": "damage", "sourceId": null, "targetId": "hero-p2", "amount": 3, "combat": false, "absorbed": 2 })
            ]
        );

        // D14: a hit no Armor touched reads on the wire exactly as it did before the field existed.
        let (_, events) = hit_alone(&mut state, None, Aim::Hero(P1), 4, None);
        assert_eq!(
            events,
            vec![
                json!({ "type": "damage", "sourceId": null, "targetId": "hero-p1", "amount": 4, "combat": false })
            ]
        );
    }

    #[test]
    fn r1360_a_hero_s_divisors_and_cap_are_not_armor_so_absorbed_is_the_armor_alone() {
        let mut state = playing("r1360-divisor");
        put(&mut state, &argus.id, slot(P1, Row::Backrow, 1), json!({})); // halves each hit on p1
        put(
            &mut state,
            &anti_oneshot().id,
            slot(P1, Row::Backrow, 2),
            json!({}),
        ); // caps it at 5
        state.players.p1.hero.armor = 1;
        let (dealt, events) = hit_alone(&mut state, None, Aim::Hero(P1), 12, None);
        // 12 − 1 Armor = 11, halved and rounded up = 6, capped at 5: the Armor took 1 of it.
        assert_eq!(dealt, 5);
        assert_eq!(
            wire(&events_of_type(&json_events(&events), GameEventType::Damage)),
            vec![
                json!({ "type": "damage", "sourceId": null, "targetId": "hero-p1", "amount": 5, "combat": false, "absorbed": 1 })
            ]
        );
    }

    #[test]
    fn r1360_a_hit_that_pierces_has_no_armor_taken_off_it_so_absorbed_is_always_0() {
        let mut state = game("r1360-pierce");
        let piercer = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        grant(&mut state, &piercer.id, Keyword::Pierce);
        let wall = put(&mut state, &armoured.id, slot(P2, Row::Units, 1), json!({})); // Armor 7

        // A unit's Pierce, read through the layers (R346): the 5 lands whole, and no `absorbed` rides it.
        let (dealt, events) = hit_alone(&mut state, Some(&piercer.id), Aim::Unit(&wall.id), 5, None);
        assert_eq!(dealt, 5);
        assert_eq!(
            events,
            vec![
                json!({ "type": "damage", "sourceId": piercer.id, "targetId": wall.id, "amount": 5, "combat": false })
            ]
        );

        // An effect's stated Pierce on a hero with Armor 3: likewise.
        state.players.p2.hero.armor = 3;
        let (dealt, events) = hit_alone(
            &mut state,
            None,
            Aim::Hero(P2),
            4,
            Some(json!({ "ignoreArmor": true })),
        );
        assert_eq!(dealt, 4);
        assert_eq!(
            events,
            vec![
                json!({ "type": "damage", "sourceId": null, "targetId": "hero-p2", "amount": 4, "combat": false })
            ]
        );

        // And a piercing hit the Armor would have taken whole is no `damageAbsorbed`: it lands.
        let (dealt, events) = hit_alone(
            &mut state,
            None,
            Aim::Hero(P2),
            2,
            Some(json!({ "ignoreArmor": true })),
        );
        assert_eq!(dealt, 2);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], json!("damage"));
    }

    #[test]
    fn r1360_a_redirected_hit_carries_the_armor_of_the_hero_it_lands_on_and_nothing_of_the_one_it_left() {
        let mut state = playing("r1360-redirect");
        put(&mut state, &gambit.id, slot(P2, Row::Backrow, 1), json!({}));
        state.players.p2.hero.armor = 3;
        state.players.p2.hero.health = 5;
        state.players.p1.hero.armor = 1;
        let (dealt, events) = hit_alone(&mut state, None, Aim::Hero(P2), 12, None);
        // 12 − 3 = 9 would bring p2 to 0, so Final Gambit's shape sends the hit on to p1 as a new
        // instance from the same source (E9), through p1's Armor 1: p2's Armor took nothing that landed.
        assert_eq!(dealt, 11);
        let typed = json_events(&events);
        assert_eq!(
            wire(&events_of_type(&typed, GameEventType::Damage)),
            vec![
                json!({ "type": "damage", "sourceId": null, "targetId": "hero-p1", "amount": 11, "combat": false, "absorbed": 1 })
            ]
        );
        assert!(events_of_type(&typed, GameEventType::DamageAbsorbed).is_empty());
        assert_eq!(state.players.p2.hero.health, 5);

        // R1361: when the hero it lands on has the Armor to take it whole, that hero reports it.
        let mut state = playing("r1361-redirect");
        put(&mut state, &gambit.id, slot(P2, Row::Backrow, 1), json!({}));
        state.players.p2.hero.health = 5;
        state.players.p1.hero.armor = 20;
        let (dealt, events) = hit_alone(&mut state, None, Aim::Hero(P2), 12, None);
        assert_eq!(dealt, 0);
        let typed = json_events(&events);
        assert!(events_of_type(&typed, GameEventType::Damage).is_empty());
        assert_eq!(
            wire(&events_of_type(&typed, GameEventType::DamageAbsorbed)),
            vec![
                json!({ "type": "damageAbsorbed", "sourceId": null, "targetId": "hero-p1", "absorbed": 12, "combat": false })
            ]
        );
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH);
        assert_eq!(state.players.p2.hero.health, 5);
    }

    #[test]
    fn r1360_a_trample_excess_is_its_own_instance_through_its_own_hero_s_armor() {
        let mut state = game("r1360-trample");
        let source = put(&mut state, &trampler.id, slot(P1, Row::Units, 1), json!({}));
        let target = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({})); // 3/3
        grant(&mut state, &target.id, Keyword::Armor { n: 1 });
        state.players.p2.hero.armor = 2;
        let (dealt, events) = hit_alone(&mut state, Some(&source.id), Aim::Unit(&target.id), 10, None);
        // 10 − 1 = 9 onto 3 health: 3 dealt and 6 trample on, of which p2's Armor takes 2 (R63).
        assert_eq!(dealt, 3);
        assert_eq!(
            events,
            vec![
                json!({ "type": "damage", "sourceId": source.id, "targetId": target.id, "amount": 3, "combat": false, "absorbed": 1 }),
                json!({ "type": "damage", "sourceId": source.id, "targetId": "hero-p2", "amount": 4, "combat": false, "absorbed": 2 }),
            ]
        );
    }
}

/// The JSON `hit_alone` returns, read back as events.
fn json_events(events: &[Value]) -> Vec<GameEvent> {
    events.iter().map(|event| json_as(event.clone())).collect()
}

mod r1361_a_hit_armor_takes_whole_is_reported_and_answered_by_nothing {
    use super::*;

    #[test]
    fn r1361_r63_the_report_replaces_no_step_lifesteal_poisonous_trample_and_kill_credit_never_run() {
        let mut state = game("r1361-whole");
        state.players.p1.hero.health = 20;
        let wall = put(&mut state, &armoured.id, slot(P2, Row::Units, 1), json!({})); // 7/7, Armor 7
        let leech = put(&mut state, &lifestealer.id, slot(P1, Row::Units, 1), json!({}));
        let viper = put(&mut state, &poisonous.id, slot(P1, Row::Units, 2), json!({}));
        let tramp = put(&mut state, &trampler.id, slot(P1, Row::Units, 3), json!({}));
        for (source, amount) in [(&leech, 7), (&viper, 1), (&tramp, 6)] {
            let (dealt, events) = hit_alone(
                &mut state,
                Some(&source.id),
                Aim::Unit(&wall.id),
                amount,
                Some(json!({ "combat": true })),
            );
            assert_eq!(dealt, 0);
            assert_eq!(
                events,
                vec![
                    json!({ "type": "damageAbsorbed", "sourceId": source.id, "targetId": wall.id, "absorbed": amount, "combat": true })
                ],
                "{amount} from {}",
                source.def_id
            );
        }
        // None of steps 5 to 9 ran: nothing dealt, no Lifesteal heal (step 8), no Poisonous mark
        // (step 7), no Trample excess on p2's hero (step 9), and no kill credit (R42).
        let now = live(&state, &wall.id);
        assert_eq!(now.damage, 0);
        assert_eq!(now.marked_destroyed, None);
        assert_eq!(now.last_damaged_by, None);
        assert_eq!(state.players.p1.hero.health, 20);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);

        // A hero's Armor that takes a hit whole reports it the same way, and its Lifesteal heals nothing.
        state.players.p2.hero.armor = 5;
        let (dealt, events) = hit_alone(&mut state, Some(&leech.id), Aim::Hero(P2), 3, None);
        assert_eq!(dealt, 0);
        assert_eq!(
            events,
            vec![
                json!({ "type": "damageAbsorbed", "sourceId": leech.id, "targetId": "hero-p2", "absorbed": 3, "combat": false })
            ]
        );
        assert_eq!(state.players.p1.hero.health, 20);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
    }

    #[test]
    fn r1361_an_indestructible_unit_s_armor_reports_what_it_takes_whole_and_divine_shield_is_no_armor() {
        let mut state = game("r1361-steps");
        let idol = put(&mut state, &indestructible.id, slot(P2, Row::Units, 1), json!({}));
        grant(&mut state, &idol.id, Keyword::Armor { n: 2 });
        // §4.4 orders the zero rule after step 3 and before step 4, so the Armor's whole stop reports...
        let (dealt, events) = hit_alone(&mut state, None, Aim::Unit(&idol.id), 2, None);
        assert_eq!(dealt, 0);
        assert_eq!(
            events,
            vec![
                json!({ "type": "damageAbsorbed", "sourceId": null, "targetId": idol.id, "absorbed": 2, "combat": false })
            ]
        );
        // ...while a hit the Armor only shrank meets Indestructible at step 4, which stops it unreported.
        let (dealt, events) = hit_alone(&mut state, None, Aim::Unit(&idol.id), 5, None);
        assert_eq!(dealt, 0);
        assert!(events.is_empty());

        // Step 1 comes first: a Divine Shield takes the hit, whatever Armor stands behind it.
        let saint = put(&mut state, &shielded.id, slot(P2, Row::Units, 2), json!({}));
        grant(&mut state, &saint.id, Keyword::Armor { n: 5 });
        let (_, events) = hit_alone(&mut state, None, Aim::Unit(&saint.id), 3, None);
        assert_eq!(
            events,
            vec![json!({ "type": "divineShieldLost", "instanceId": saint.id })]
        );
        // A hit of 0 is no instance at all (R63): nothing to absorb and nothing reported.
        let (_, events) = hit_alone(&mut state, None, Aim::Unit(&saint.id), 0, None);
        assert!(events.is_empty());
    }

    /// A Unit gaining a Plague Counter whenever it is hit (Fed Fauci's shape, §4.4 step 6) and a Trap
    /// that heals its hero 5 when that Unit is hit, each listening for the report as well as the hit.
    fn register_listeners() -> (CardDef, CardDef) {
        let unit: CardDef = json_as(json!({
            "id": "ab-listener",
            "index": "991",
            "name": "Hit Listener (armor fixture)",
            "set": "Core",
            "type": "Unit",
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 2,
            "base": { "attack": 1, "health": 6, "keywords": [], "text": "+1 Plague Counter when this is hit" },
            "radiant": { "attack": 2, "health": 12, "keywords": [], "text": "same" },
        }));
        let trap: CardDef = json_as(json!({
            "id": "ab-trap",
            "index": "992",
            "name": "Hit Trap (armor fixture)",
            "set": "Core",
            "type": "Trap",
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "keywords": [], "text": "Reveals when the listener is hit: heal your hero 5" },
            "radiant": { "keywords": [], "text": "same" },
        }));
        let mut catalog = registered_catalog().clone();
        catalog.insert(unit.id.clone(), unit.clone());
        catalog.insert(trap.id.clone(), trap.clone());
        register_catalog(catalog);

        let hits = [GameEventType::Damage, GameEventType::DamageAbsorbed];
        let on_me = |event: &GameEvent, me: &str| match event {
            GameEvent::Damage { target_id, .. } | GameEvent::DamageAbsorbed { target_id, .. } => {
                target_id == me
            }
            _ => false,
        };
        let listener = Script {
            triggers: vec![TriggerDef::new(
                "plague-on-hit",
                &hits,
                move |ctx, event| match ctx.self_.as_ref() {
                    Some(me) if on_me(event, &me.id) => vec![plague(json_as(json!({ "amount": 1 })))],
                    _ => vec![],
                },
            )],
            ..Script::default()
        };
        let springs = Script {
            triggers: vec![
                TriggerDef::new("heal-on-hit", &hits, |_ctx, _event| {
                    vec![heal(json_as(
                        json!({ "target": { "of": "selfHero" }, "amount": 5 }),
                    ))]
                })
                .with_when(move |_ctx, event| {
                    matches!(
                        event,
                        GameEvent::Damage { target_id, .. } | GameEvent::DamageAbsorbed { target_id, .. }
                            if !target_id.starts_with("hero-")
                    )
                }),
            ],
            ..Script::default()
        };
        let mut scripts = registered_scripts().clone();
        scripts.insert(
            unit.id.clone(),
            CardScripts {
                base: listener.clone(),
                radiant: listener,
            },
        );
        scripts.insert(
            trap.id.clone(),
            CardScripts {
                base: springs.clone(),
                radiant: springs,
            },
        );
        register_scripts(scripts);
        (unit, trap)
    }

    #[test]
    fn r1361_r63_no_on_damage_trigger_and_no_trap_answers_the_report_though_both_answer_a_hit() {
        let mut state = game("r1361-unanswered");
        let (unit, trap) = register_listeners();
        let listener = put(&mut state, &unit.id, slot(P2, Row::Units, 1), json!({}));
        grant(&mut state, &listener.id, Keyword::Armor { n: 7 });
        let set = put(&mut state, &trap.id, slot(P2, Row::Backrow, 1), json!({}));
        state.players.p2.hero.health = 20;

        with_sink(&mut state, |sink| {
            // The Armor takes the 4 whole: reported, and answered by nothing (R63, `dispatch_event`).
            assert_eq!(hit(sink, None, Aim::Unit(&listener.id), 4, None), 0);
            settle(sink, Default::default());
            assert_eq!(
                wire(&events_of_type(sink.events, GameEventType::DamageAbsorbed)),
                vec![
                    json!({ "type": "damageAbsorbed", "sourceId": null, "targetId": listener.id, "absorbed": 4, "combat": false })
                ]
            );
            assert_eq!(live(sink.state, &listener.id).counters.plague, None);
            assert_eq!(sink.state.players.p2.hero.health, 20);
            assert_eq!(
                card_at(sink.state, slot(P2, Row::Backrow, 1)).map(|card| card.id.clone()),
                Some(set.id.clone())
            );
            assert!(events_of_type(sink.events, GameEventType::TrapFired).is_empty());

            // The control: a hit that lands wakes both (§4.4 step 6, §10.3).
            assert_eq!(hit(sink, None, Aim::Unit(&listener.id), 9, None), 2);
            settle(sink, Default::default());
            assert_eq!(live(sink.state, &listener.id).counters.plague, Some(1));
            assert_eq!(sink.state.players.p2.hero.health, 25);
            assert_eq!(events_of_type(sink.events, GameEventType::TrapFired).len(), 1);
        });
    }

    #[test]
    fn r1361_an_attack_into_armor_that_takes_it_whole_is_reported_as_combat_and_the_defender_still_strikes_back()
     {
        let mut state = playing("r1361-combat");
        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({})); // 3/3
        let wall = put(&mut state, &armoured.id, slot(P2, Row::Units, 1), json!({})); // 7/7, Armor 7
        let action = json_as::<ActionInput>(
            json!({ "type": "attack", "attackerId": attacker.id, "targetId": wall.id, "playerId": "p1" }),
        )
        .with_nonce("r1361-attack");
        let result = reduce(&state, &action);
        assert_eq!(result.error, None);
        assert_eq!(
            wire(&events_of_type(&result.events, GameEventType::DamageAbsorbed)),
            vec![
                json!({ "type": "damageAbsorbed", "sourceId": attacker.id, "targetId": wall.id, "absorbed": 3, "combat": true })
            ]
        );
        assert_eq!(
            wire(&events_of_type(&result.events, GameEventType::Damage)),
            vec![
                json!({ "type": "damage", "sourceId": wall.id, "targetId": attacker.id, "amount": 7, "combat": true })
            ]
        );
        assert_eq!(live(&result.state, &wall.id).damage, 0);
        assert!(
            find_instance(&result.state, &attacker.id).is_none_or(|card| card.zone.z() != ZoneName::Field)
        );
    }

    #[test]
    fn r1361_r97_the_report_names_and_hides_what_damage_does_in_each_seat_s_view() {
        let mut state = game("r1361-view");
        // A card in p1's hand dealt the hits: p1 reads it, p2 may not (R97).
        let source = in_hand(&mut state, &plain.id, P1, 1).remove(0);
        let wall = put(&mut state, &armoured.id, slot(P2, Row::Units, 1), json!({}));
        state.applied.push(AppliedAction {
            nonce: "r1361-view".into(),
            events: vec![
                GameEvent::DamageAbsorbed {
                    source_id: Some(source.id.clone()),
                    target_id: wall.id.clone(),
                    absorbed: 3,
                    combat: false,
                },
                GameEvent::Damage {
                    source_id: Some(source.id.clone()),
                    target_id: wall.id.clone(),
                    amount: 1,
                    combat: false,
                    absorbed: 7,
                },
            ],
        });
        let seen = |viewer: PlayerId| -> Vec<Value> {
            wire(&view_for(&state, viewer).events)
                .into_iter()
                .filter(|event| event["targetId"] == json!(wall.id))
                .collect()
        };
        assert_eq!(
            seen(P1),
            vec![
                json!({ "type": "damageAbsorbed", "sourceId": source.id, "targetId": wall.id, "absorbed": 3, "combat": false }),
                json!({ "type": "damage", "sourceId": source.id, "targetId": wall.id, "amount": 1, "combat": false, "absorbed": 7 }),
            ]
        );
        assert_eq!(
            seen(P2),
            vec![
                json!({ "type": "damageAbsorbed", "sourceId": HIDDEN_ID, "targetId": wall.id, "absorbed": 3, "combat": false }),
                json!({ "type": "damage", "sourceId": HIDDEN_ID, "targetId": wall.id, "amount": 1, "combat": false, "absorbed": 7 }),
            ]
        );
    }
}

/// What the reports cost the state: nothing a rule reads, and no number in R68's order (D14, R768).
mod r1360_r1361_r1362_the_reports_leave_the_state_as_it_was {
    use super::*;

    #[test]
    fn r1360_r1361_the_frontier_holds_the_hits_as_the_rules_read_them() {
        let hit = GameEvent::Damage {
            source_id: Some("c3".into()),
            target_id: "c8".into(),
            amount: 2,
            combat: true,
            absorbed: 4,
        };
        assert_eq!(
            hit.as_rules_read(),
            GameEvent::Damage {
                source_id: Some("c3".into()),
                target_id: "c8".into(),
                amount: 2,
                combat: true,
                absorbed: 0,
            }
        );
        // R63: a hit Armor took whole is, to the rules, a hit of 0, which nothing answers.
        let report = GameEvent::DamageAbsorbed {
            source_id: None,
            target_id: "hero-p1".into(),
            absorbed: 3,
            combat: false,
        };
        assert_eq!(
            report.as_rules_read(),
            GameEvent::Damage {
                source_id: None,
                target_id: "hero-p1".into(),
                amount: 0,
                combat: false,
                absorbed: 0,
            }
        );
        let other = GameEvent::DrawOffered { player: P1 };
        assert_eq!(other.as_rules_read(), other);
    }

    #[test]
    fn r1361_r1362_a_report_takes_no_number_in_r68_s_order_and_an_absorbed_fatigue_draw_keeps_r240_s_one() {
        let mut state = game("r1361-seq");
        let wall = put(&mut state, &armoured.id, slot(P2, Row::Units, 1), json!({})); // Armor 7
        let seq = state.next_seq;
        with_sink(&mut state, |sink| {
            // A hit the Armor takes whole: reported, and the frontier takes nothing.
            assert_eq!(hit(sink, None, Aim::Unit(&wall.id), 3, None), 0);
            settle(sink, Default::default());
            assert_eq!(sink.state.next_seq, seq);
            assert!(sink.state.dispatch.is_empty());
            // A hit that lands takes its number, as every event the loop dispatches does.
            assert_eq!(hit(sink, None, Aim::Unit(&wall.id), 9, None), 2);
            settle(sink, Default::default());
            assert_eq!(sink.state.next_seq, seq + 1);
        });

        // R1362: an absorbed fatigue draw's `fatigue` and its report take two numbers, as the `fatigue`
        // and R240's `damage` of 0 did.
        let mut state = game("r1362-seq");
        state.players.p1.library = Vec::new();
        state.players.p1.hero.armor = 3;
        let seq = state.next_seq;
        with_sink(&mut state, |sink| {
            assert!(matches!(draw::draw_one(sink, P1, None), DrawOutcome::Fatigue));
            settle(sink, Default::default());
            assert_eq!(
                wire(sink.events),
                vec![
                    json!({ "type": "fatigue", "player": "p1", "count": 1, "amount": 1 }),
                    json!({ "type": "damageAbsorbed", "sourceId": null, "targetId": "hero-p1", "absorbed": 1, "combat": false }),
                ]
            );
            assert_eq!(sink.state.next_seq, seq + 2);
            assert_eq!(sink.state.players.p1.hero.health, HERO_HEALTH);
        });
    }
}
