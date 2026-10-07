//! C+ #22 Blood Moon (SPEC §8.7 row 22): the "would be healed" replacement (B5 E5, E8, R413): an enemy heal
//! becomes Pierce damage from it, for the rest of the turn; the Radiant face is a Field Trap that keeps it.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-022";

/// `{ id: "blood-moon", on: "healed", instead }`: the one replacement both faces declare.
fn blood_moon(instead: ReplacementInstead) -> ReplacementDef {
    ReplacementDef {
        id: "blood-moon".to_string(),
        on: ReplacementMoment::Healed,
        where_: None,
        when: None,
        instead,
        then: None,
        by: None,
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        replacements: vec![blood_moon(ReplacementInstead {
            damage: Some(InsteadDamage::Pierce),
            lasting: Some(InsteadLasting::ThisTurn),
            ..ReplacementInstead::default()
        })],
        ..Script::default()
    };

    let radiant = Script {
        static_flags: Some(StaticFlags {
            heal_to_damage: Some(true),
            ..StaticFlags::default()
        }),
        replacements: vec![blood_moon(ReplacementInstead {
            damage: Some(InsteadDamage::Pierce),
            ..ReplacementInstead::default()
        })],
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C+ #22 Blood Moon — SPEC §8.7 row 22, BUILD M9 Classic+ row C+ 22: "Face-down Trap in the "would be
// healed" replacement: it fires when an enemy (the other player's hero or one of their Units) would be
// healed, by a heal, Lifesteal, "heal up to" or "heal to full"; the heal that sets it off is converted
// too (R413), and every heal of X on an enemy for the rest of the turn becomes X Pierce damage from
// Blood Moon (Armor skipped, Divine Shield and caps still apply, no Spell Damage), the modifier ending
// at cleanup while the trap lies in the graveyard; Set health is no heal; a friendly heal never fires
// it and the opponent learns nothing of it until it fires (R33, R97); radiant its face is a Field Trap,
// in hand too (pools and filters read Field Trap), it stays face-up once fired and converts every
// enemy heal for as long as it is on the field".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MOON: &str = "classicplus-022";
    const FIG: &str = "core-047"; // Heal a target 20.
    const RENO: &str = "core-053"; // Cry: heal your hero up to 30.
    const MENACE: &str = "core-019"; // 9/9 Taunt; end of turn: heal this to full.
    const JILLIAX: &str = "core-056"; // 3/2 Rush, Taunt, Lifesteal, Divine Shield
    const ANTI_ONESHOT: &str = "core-073"; // the hero takes at most 5 at once
    const SOLARIUS: &str = "classicplus-038"; // Spell Damage +2
    const NETHER: &str = "core-088"; // Twisting Nether: destroy all permanents
    const FILLER: &str = "core-005";
    const DECK: [&str; 6] = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];

    fn enemy_hero() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    /// TS `{ ...defaults, ...overrides }` on a side setup: every key of `overrides` replaces the default's.
    fn spread(defaults: Value, overrides: &Value) -> Value {
        let mut out = defaults;
        if let (Some(into), Some(from)) = (out.as_object_mut(), overrides.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        out
    }

    /// p1 holds Blood Moon face-down in backrow lane 1; p2 is the active enemy who heals.
    fn moon_up(radiant_face: bool, p2: Value, p1: Value) -> Scenario {
        crate::register_all();
        let mut side1 = spread(json!({ "hand": [FILLER], "library": DECK }), &p1);
        let mut backrow = vec![json!({ "def": MOON, "lane": 1, "faceUp": false, "radiant": radiant_face })];
        backrow.extend(p1.get("backrow").and_then(Value::as_array).cloned().unwrap_or_default());
        side1["backrow"] = Value::Array(backrow);
        let side2 = spread(json!({ "hand": [FILLER], "library": DECK }), &p2);
        scenario(json!({ "active": "p2", "p1": side1, "p2": side2 }))
    }

    fn fired(s: &Scenario) -> bool {
        s.events()
            .iter()
            .any(|event| matches!(event, GameEvent::TrapFired { def_id, .. } if def_id == MOON))
    }

    /// The `damage` events whose source is a Blood Moon that fired: each one's amount.
    fn moon_hits(s: &Scenario) -> Vec<i32> {
        let moon_ids: IndexSet<String> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::TrapFired { def_id, instance_id, .. } if def_id == MOON => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { source_id: Some(source), amount, .. } if moon_ids.contains(source) => {
                    Some(*amount)
                }
                _ => None,
            })
            .collect()
    }

    /// The two Figs in p2's hand (TS `const [first, second] = …`).
    fn two_figs(s: &Scenario) -> (CardInstance, CardInstance) {
        let figs: Vec<CardInstance> = s.hand(P2).into_iter().filter(|card| card.def_id == FIG).collect();
        match (figs.first(), figs.get(1)) {
            (Some(first), Some(second)) => (first.clone(), second.clone()),
            _ => panic!("two Figs"),
        }
    }

    /// TS `s.unit(p, lane) ?? ""`: the unit's id, or a reference that names nothing.
    fn unit_or_blank(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane).map(|unit| unit.id).unwrap_or_default()
    }

    use crate::matches_object;

    use crate::js;

    mod c_n22_blood_moon {
        use super::*;

        #[test]
        fn is_a_1_trap_whose_radiant_face_is_a_field_trap_both_faces_replace_a_heal() {
            let def = crate::card_def(ID);
            assert_eq!(def.type_, CardType::Trap);
            assert_eq!(def.radiant.type_, Some(CardType::FieldTrap));
            let scripts = script();
            let first = scripts.base.replacements.first().expect("a replacement");
            assert!(matches_object(
                &json!({ "on": first.on, "instead": js(&first.instead) }),
                &json!({ "on": "healed", "instead": { "damage": "pierce", "lasting": "thisTurn" } }),
            ));
            let first = scripts.radiant.replacements.first().expect("a replacement");
            assert!(matches_object(
                &json!({ "on": first.on, "instead": js(&first.instead) }),
                &json!({ "on": "healed", "instead": { "damage": "pierce" } }),
            ));
            assert_eq!(scripts.radiant.static_flags.as_ref().and_then(|flags| flags.heal_to_damage), Some(true));
        }

        mod base {
            use super::*;

            #[test]
            fn r413_an_enemy_heal_fires_it_and_is_itself_converted_20_healing_becomes_20_pierce_damage() {
                let mut s = moon_up(false, json!({ "hand": [FIG, FILLER] }), json!({}));
                s.play(FIG, json!({ "targets": enemy_hero() }));
                assert!(fired(&s));
                s.expect_health(P2, 10);
                assert_eq!(moon_hits(&s), vec![20]);
                s.expect_in_zone(MOON, "graveyard");
                assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Healed));
            }

            #[test]
            fn r413_a_heal_on_an_enemy_unit_converts_the_same_way() {
                let mut s = moon_up(
                    false,
                    json!({ "hand": [FIG, FILLER], "field": [{ "def": MENACE, "lane": 1, "damage": 4 }] }),
                    json!({}),
                );
                let menace = s.unit(P2, 1).expect("setup");
                s.play(FIG, json!({ "targets": [{ "pick": "instance", "instanceId": menace.id }] }));
                assert_eq!(s.card(&menace).zone.z(), ZoneName::Graveyard);
            }

            #[test]
            fn s6_1_lifesteal_is_healing_the_attacker_s_lifesteal_heal_becomes_damage_to_its_own_hero() {
                let mut s = moon_up(false, json!({ "field": [{ "def": JILLIAX, "lane": 1 }] }), json!({}));
                let attacker = unit_or_blank(&s, P2, 1);
                s.attack(attacker, "hero");
                s.expect_health(P1, 27);
                assert!(fired(&s));
                s.expect_health(P2, 27);
            }

            #[test]
            fn s6_3_heal_up_to_converts_what_it_would_restore() {
                let mut s = moon_up(false, json!({ "hand": [RENO, FILLER], "health": 22 }), json!({}));
                s.play(RENO, json!({}));
                assert!(fired(&s));
                s.expect_health(P2, 14);
            }

            #[test]
            fn s6_3_heal_to_full_converts_what_it_would_restore() {
                let mut s = moon_up(false, json!({ "field": [{ "def": MENACE, "lane": 1, "damage": 3 }] }), json!({}));
                let menace = s.unit(P2, 1).expect("setup");
                s.end_turn();
                assert!(fired(&s));
                assert_eq!(s.card(&menace).damage, 6);
            }

            #[test]
            fn every_enemy_heal_for_the_rest_of_that_turn_converts_the_trap_already_in_the_graveyard() {
                let mut s = moon_up(false, json!({ "hand": [FIG, FIG, FILLER], "mana": 6 }), json!({}));
                let (first, second) = two_figs(&s);
                s.play(&first, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 10);
                s.play(&second, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
                s.expect_health(P2, -10);
            }

            #[test]
            fn the_conversion_ends_at_that_turn_s_cleanup_the_next_turn_s_heal_heals() {
                let mut s = moon_up(false, json!({ "hand": [FIG, FIG, FILLER], "health": 40 }), json!({}));
                let (first, second) = two_figs(&s);
                s.play(&first, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 20);
                s.end_turn().end_turn();
                s.play(&second, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 40);
            }

            #[test]
            fn r346_the_damage_is_pierce_the_hero_s_armor_is_skipped() {
                let mut s = moon_up(false, json!({ "hand": [FIG, FILLER], "armor": 5 }), json!({}));
                s.play(FIG, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 10);
            }

            #[test]
            fn s4_4_a_hit_cap_still_applies_anti_oneshot_armor_holds_the_20_to_5() {
                let mut s = moon_up(
                    false,
                    json!({ "hand": [FIG, FILLER], "backrow": [{ "def": ANTI_ONESHOT, "lane": 2 }] }),
                    json!({}),
                );
                s.play(FIG, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 25);
            }

            #[test]
            fn s4_4_divine_shield_still_applies_a_shielded_enemy_unit_loses_its_shield_and_takes_nothing() {
                let mut s = moon_up(
                    false,
                    json!({ "hand": [FIG, FILLER], "field": [{ "def": JILLIAX, "lane": 1, "damage": 1 }] }),
                    json!({}),
                );
                let jilliax = s.unit(P2, 1).expect("setup");
                s.play(FIG, json!({ "targets": [{ "pick": "instance", "instanceId": jilliax.id }] }));
                assert_eq!(s.card(&jilliax).damage, 1);
                assert!(!s.stats(&jilliax).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::DivineShield));
            }

            #[test]
            fn b5_e6_no_spell_damage_a_trap_s_damage_is_not_a_spell_s() {
                let mut s = moon_up(
                    false,
                    json!({ "hand": [FIG, FILLER] }),
                    json!({ "field": [{ "def": SOLARIUS, "lane": 1 }] }),
                );
                s.play(FIG, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 10);
            }

            #[test]
            fn a_friendly_heal_never_fires_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p1",
                    "p1": {
                        "hand": [FIG, FILLER],
                        "health": 5,
                        "backrow": [{ "def": MOON, "lane": 1, "faceUp": false }],
                        "library": DECK,
                    },
                    "p2": { "hand": [FILLER], "library": DECK },
                }));
                s.play(FIG, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                assert!(!fired(&s));
                s.expect_health(P1, 25);
                assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some(MOON.to_string()));
            }

            #[test]
            fn s6_3_set_health_is_no_heal_it_does_not_fire() {
                let mut s = moon_up(false, json!({ "health": 5 }), json!({}));
                let (seed, cursor) = (s.state().seed.clone(), s.state().rng_cursor);
                let mut events: Vec<GameEvent> = Vec::new();
                let mut rng = Rng::new(&seed, cursor);
                {
                    let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                    let mut ctx = make_context(
                        &mut sink,
                        None,
                        HookOptions { controller: Some(P2), ..HookOptions::default() },
                    );
                    apply_effects(
                        &[set_health(json_as(json!({ "to": { "of": "selfHero" }, "value": 13 })))],
                        &mut ctx,
                    );
                }
                assert!(!events.iter().any(|event| event.event_type() == GameEventType::TrapFired));
                s.expect_health(P2, 13);
                assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some(MOON.to_string()));
            }

            #[test]
            fn r33_r97_the_opponent_learns_nothing_of_it_until_it_fires() {
                let mut s = moon_up(false, json!({ "hand": [FIG, FILLER] }), json!({}));
                let before = s.view(P2).opponent.backrow.first().cloned().flatten();
                assert!(matches_object(&js(&before), &json!({ "faceDown": true })));
                assert!(!serde_json::to_string(&before).expect("JSON").contains(MOON));
                assert!(!serde_json::to_string(&s.view(P2)).expect("JSON").contains(MOON));
                s.play(FIG, json!({ "targets": enemy_hero() }));
                assert!(serde_json::to_string(&s.view(P2).opponent.graveyard).expect("JSON").contains(MOON));
            }

            #[test]
            fn the_state_after_it_fired_is_plain_json() {
                let mut s = moon_up(false, json!({ "hand": [FIG, FILLER] }), json!({}));
                s.play(FIG, json!({ "targets": enemy_hero() }));
                let round: GameState = serde_json::from_value(js(s.state())).expect("a state");
                assert_eq!(&round, s.state());
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn b2_7_its_face_is_a_field_trap_in_hand_too() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [{ "def": MOON, "radiant": true }, MOON, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                let moons: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| card.def_id == MOON).collect();
                let (shining, plain) = match (moons.first(), moons.get(1)) {
                    (Some(shining), Some(plain)) => (shining.clone(), plain.clone()),
                    _ => panic!("setup"),
                };
                assert_eq!(card_type_of(s.state(), &shining), CardType::FieldTrap);
                assert_eq!(card_type_of(s.state(), &plain), CardType::Trap);
                let hand = match s.view(P1).you.hand {
                    HandView::Cards(cards) => cards,
                    HandView::Count { .. } => panic!("own hand"),
                };
                assert_eq!(
                    hand.iter().find(|card| card.instance_id == shining.id).and_then(|card| card.type_),
                    Some(CardType::FieldTrap)
                );
            }

            #[test]
            fn r413_fires_on_an_enemy_heal_converts_it_and_stays_face_up_on_the_field() {
                let mut s = moon_up(true, json!({ "hand": [FIG, FILLER] }), json!({}));
                s.play(FIG, json!({ "targets": enemy_hero() }));
                assert!(fired(&s));
                s.expect_health(P2, 10);
                let moon = s.backrow(P1, 1);
                assert_eq!(moon.as_ref().map(|card| card.def_id.as_str()), Some(MOON));
                assert_eq!(moon.as_ref().and_then(|card| card.face_up), Some(true));
            }

            #[test]
            fn keeps_converting_every_enemy_heal_on_later_turns_while_it_stays() {
                let mut s = moon_up(true, json!({ "hand": [FIG, FIG, FILLER], "health": 60 }), json!({}));
                let (first, second) = two_figs(&s);
                s.play(&first, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 40);
                s.end_turn().end_turn();
                s.play(&second, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 20);
                assert_eq!(
                    s.events().iter().filter(|event| event.event_type() == GameEventType::TrapFired).count(),
                    1
                );
            }

            #[test]
            fn r33_r97_face_down_the_opponent_learns_nothing_of_it_not_even_that_it_is_a_field_trap() {
                let s = moon_up(true, json!({ "hand": [FIG, FILLER] }), json!({}));
                let first = s.view(P2).opponent.backrow.first().cloned().flatten();
                assert_eq!(js(&first), json!({ "faceDown": true, "cost": 1 }));
                let seen = serde_json::to_string(&s.view(P2)).expect("JSON");
                assert!(!seen.contains(MOON));
                assert!(!seen.contains("Field Trap"));
            }

            #[test]
            fn converts_only_while_it_is_on_the_field_destroyed_the_next_enemy_heal_heals() {
                let mut s = moon_up(
                    true,
                    json!({ "hand": [FIG, FIG, NETHER, FILLER], "health": 60, "mana": 10 }),
                    json!({}),
                );
                let (first, second) = two_figs(&s);
                s.play(&first, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 40);
                let moon = s.backrow(P1, 1).expect("the Field Trap left");
                s.play(NETHER, json!({}));
                s.expect_in_zone(&moon, "graveyard");
                s.play(&second, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 60);
            }

            #[test]
            fn a_friendly_heal_is_untouched_while_it_stands_face_up() {
                let mut s = moon_up(
                    true,
                    json!({ "hand": [FIG, FILLER] }),
                    json!({ "hand": [FIG, FILLER], "health": 5 }),
                );
                s.play(FIG, json!({ "targets": enemy_hero() }));
                s.end_turn();
                s.play(FIG, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                s.expect_health(P1, 25);
            }
        }
    }
}
