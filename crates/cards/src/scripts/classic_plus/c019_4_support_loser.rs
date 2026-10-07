//! C+ #19.4 Support Loser (SPEC §8.7 row 19.4): end of turn, heal your hero, then each of your Units,
//! {heal}.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-019-4";

pub fn script() -> CardScripts {
    let base = Script {
        end_of_turn: Some(hook(|ctx| {
            let amount = param(&*ctx, "heal");
            vec![
                heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": amount }))),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|each: &mut EffectContext<'_>| {
                        cards_in_scope(each, &json_as(json!({ "side": "self" })))
                            .into_iter()
                            .map(|card| card.id)
                            .collect()
                    }),
                    each: Arc::new(move |instance_id: &str| {
                        heal(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "amount": amount,
                        })))
                    }),
                }),
            ]
        })),
        ..Script::default()
    };

    // The same script: Reborn and the Radiant heal 6 are catalog data.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C+ #19.4 Support Loser — SPEC §8.7 row 19.4, BUILD M9 Classic+ row C+ 19.4: "Divine Shield, 0 attack,
// so it never declares an attack and strikes back with no damage instance (R63); at its controller's
// end of turn heals your hero 3 (past 30 allowed) and each of your Units 3 up to max health, itself
// included; an opposing Blood Moon turns each of those heals into Pierce damage (R413); the heal reads
// through `param()`; radiant 0/10 Divine Shield, Reborn, heal 6".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SUPPORT: &str = "classicplus-019-4";
    const BODY: &str = "core-019"; // 9/9 Taunt; end of turn: heal this to full — kept off the board here
    const VANILLA: &str = "core-008"; // 4/4
    const MOON: &str = "classicplus-022";
    const HIT_JOB: &str = "core-016";
    const FILLER: &str = "core-005";
    const DECK: [&str; 4] = [FILLER, FILLER, FILLER, FILLER];

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

    fn support(p1: Value, radiant_face: bool, p2: Value) -> Scenario {
        crate::register_all();
        let mut side1 = spread(json!({ "hand": [FILLER], "library": DECK }), &p1);
        let mut field = vec![json!({ "def": SUPPORT, "lane": 3, "radiant": radiant_face })];
        field.extend(p1.get("field").and_then(Value::as_array).cloned().unwrap_or_default());
        side1["field"] = Value::Array(field);
        let side2 = spread(json!({ "hand": [FILLER], "library": DECK }), &p2);
        scenario(json!({ "p1": side1, "p2": side2 }))
    }

    fn loser(s: &Scenario) -> CardInstance {
        match s.unit(P1, 3) {
            Some(unit) if unit.def_id == SUPPORT => unit,
            _ => panic!("no Support Loser in lane 3"),
        }
    }

    /// TS wrote through the live instance `loser(s)` handed back.
    fn loser_mut(s: &mut Scenario) -> &mut CardInstance {
        let id = loser(s).id;
        find_instance_mut(s.state_mut(), &id).expect("no Support Loser in lane 3")
    }

    /// TS `s.unit(p, lane) ?? ""`: the unit's id, or a reference that names nothing.
    fn unit_or_blank(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane).map(|unit| unit.id).unwrap_or_default()
    }

    fn has_keyword(s: &Scenario, card: &str, kind: KeywordKind) -> bool {
        s.stats(card).keywords.iter().any(|keyword| keyword.kind() == kind)
    }

    mod c_n19_4_support_loser {
        use super::*;

        #[test]
        fn is_a_2_0_5_divine_shield_unit_token_printed_legendary_one_script_serves_both_faces() {
            let def = crate::card_def(ID);
            assert_eq!(def.base.attack, Some(0));
            assert_eq!(def.base.keywords, vec![Keyword::DivineShield]);
            assert_eq!(def.radiant.keywords, vec![Keyword::DivineShield, Keyword::Reborn]);
            let scripts = script();
            assert!(Arc::ptr_eq(
                scripts.base.end_of_turn.as_ref().expect("base end of turn"),
                scripts.radiant.end_of_turn.as_ref().expect("radiant end of turn"),
            ));
        }

        mod base {
            use super::*;

            #[test]
            fn s4_1_with_0_attack_it_never_declares_an_attack() {
                let s = support(json!({}), false, json!({}));
                let me = loser(&s).id;
                assert!(!legal_actions(s.state(), P1).iter().any(
                    |action| matches!(action, ActionBody::Attack { attacker_id, .. } if *attacker_id == me)
                ));
            }

            #[test]
            fn r63_attacked_it_strikes_back_with_no_damage_instance() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [FILLER], "field": [{ "def": SUPPORT, "lane": 3 }], "library": DECK },
                    "p2": { "hand": [FILLER], "field": [{ "def": VANILLA, "lane": 3 }], "library": DECK },
                }));
                let attacker = s.unit(P2, 3).expect("setup");
                let me = loser(&s);
                s.attack(&attacker, &me);
                let me = loser(&s);
                assert!(!s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::Damage { source_id: Some(source), .. } if *source == me.id
                )));
                // Its Divine Shield took the hit.
                s.expect_stats(&me, json!({ "health": 5 }));
                assert!(!has_keyword(&s, &me.id, KeywordKind::DivineShield));
            }

            #[test]
            fn at_its_controller_s_end_of_turn_heals_the_hero_3_past_30() {
                let mut s = support(json!({}), false, json!({}));
                s.end_turn();
                s.expect_health(P1, 33);
            }

            #[test]
            fn heals_each_of_your_units_3_up_to_max_health_itself_included_and_no_enemy() {
                let mut s = support(
                    json!({ "field": [{ "def": VANILLA, "lane": 1, "damage": 2 }, { "def": BODY, "lane": 5, "damage": 5 }] }),
                    false,
                    json!({ "field": [{ "def": VANILLA, "lane": 1, "damage": 3 }] }),
                );
                loser_mut(&mut s).damage = 4;
                s.end_turn();
                let first = unit_or_blank(&s, P1, 1);
                s.expect_stats(&first, json!({ "health": 4 }));
                let fifth = unit_or_blank(&s, P1, 5);
                s.expect_stats(&fifth, json!({ "health": 9 }));
                let me = loser(&s);
                s.expect_stats(&me, json!({ "health": 4 }));
                let enemy = unit_or_blank(&s, P2, 1);
                s.expect_stats(&enemy, json!({ "health": 1 }));
            }

            #[test]
            fn nothing_at_the_opponent_s_end_of_turn() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [FILLER], "field": [{ "def": SUPPORT, "lane": 3 }], "library": DECK, "health": 20 },
                    "p2": { "hand": [FILLER], "library": DECK },
                }));
                s.end_turn();
                // p1's own start of turn follows; no heal came at p2's end.
                s.expect_health(P1, 20);
            }

            #[test]
            fn r413_an_opposing_blood_moon_turns_each_heal_into_pierce_damage() {
                let mut s = support(
                    json!({ "field": [{ "def": VANILLA, "lane": 1, "damage": 2 }], "health": 20 }),
                    false,
                    json!({ "backrow": [{ "def": MOON, "lane": 1, "faceUp": false }] }),
                );
                let vanilla = s.unit(P1, 1).expect("setup");
                s.end_turn();
                // The hero heal fires the trap and is converted, and so is each Unit's heal that turn: the damaged
                // 4/4 (2 health left) takes 3 and dies, and the Support Loser's own Divine Shield takes its 3.
                s.expect_health(P1, 17);
                s.expect_in_zone(&vanilla, "graveyard");
                let me = loser(&s);
                s.expect_stats(&me, json!({ "health": 5 }));
                assert!(!has_keyword(&s, &me.id, KeywordKind::DivineShield));
                assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Healed));
            }

            #[test]
            fn r386_the_heal_reads_through_param_an_upgrade_heals_4() {
                let mut s = support(json!({}), false, json!({}));
                step_param(loser_mut(&mut s), "heal", 1);
                s.end_turn();
                s.expect_health(P1, 34);
            }

            #[test]
            fn a_token_it_ceases_to_exist_when_destroyed() {
                let mut s = support(json!({ "hand": [HIT_JOB, FILLER] }), false, json!({}));
                let unit = loser(&s);
                // Divine Shield does not stop a destroy.
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": unit.id }] }));
                s.expect_in_zone(&unit, "gone");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_0_10_with_divine_shield_and_reborn_and_heals_6() {
                let mut s = support(json!({ "field": [{ "def": VANILLA, "lane": 1, "damage": 3 }] }), true, json!({}));
                let me = loser(&s);
                s.expect_stats(&me, json!({ "attack": 0, "health": 10 }));
                s.end_turn();
                s.expect_health(P1, 36);
                let first = unit_or_blank(&s, P1, 1);
                s.expect_stats(&first, json!({ "health": 4 }));
            }

            #[test]
            fn r11_r175_reborn_brings_the_token_back_once_at_1_health_and_without_reborn() {
                let mut s = support(json!({ "hand": [HIT_JOB, FILLER] }), true, json!({}));
                let unit = loser(&s);
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": unit.id }] }));
                let back = s.unit(P1, 3);
                assert_eq!(back.as_ref().map(|card| card.def_id.as_str()), Some(SUPPORT));
                let back = back.map(|card| card.id).unwrap_or_default();
                s.expect_stats(&back, json!({ "health": 1 }));
                assert!(!has_keyword(&s, &back, KeywordKind::Reborn));
            }
        }
    }
}
