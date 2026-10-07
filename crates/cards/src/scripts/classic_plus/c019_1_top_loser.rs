//! C+ #19.1 Top Loser (SPEC §8.7 row 19.1): Armor 3 (Radiant: 6, Immune to Spells, catalog keywords);
//! never in Defense Position, and only a Unit in its lane may attack it (E35 static flags).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-019-1";

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            attacked_only_from_lane: Some(true),
            never_defense: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };

    // The same script: Armor 6 and Immune to Spells are the Radiant face's catalog keywords.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C+ #19.1 Top Loser — SPEC §8.7 row 19.1, BUILD M9 Classic+ row C+ 19.1: "Armor 3; cannot be in
// Defense Position (a switch is refused, a switch-all effect leaves it in Attack); only an enemy Unit
// in its own lane's unit zone may attack it, declared or forced: a forced attack from another lane
// does not happen and a random-enemy forced attack from another lane never draws it (§4.2 step 2); a
// Taunt given to it binds only the attacker in its lane; effects still target it; a Token, it ceases
// to exist when it leaves; its Armor reads through `param()`; radiant 10/10 Armor 6 and Immune to
// Spells: no Spell targets it and no Spell's effect touches it (Powder Spray, Whirlwind, Brawl pass it
// by), while Field Spells, Traps and Units still reach it and it may still be attacked from its lane".
//
// Its Armor is a numbered keyword, which B3.4's X change tunes rather than a declared param (R386):
// the "reads through param()" clause is proved by an Upgrade of that number.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TOP: &str = "classicplus-019-1";
    const JUNGLE: &str = "classicplus-019-2";
    const BODY: &str = "core-019"; // 9/9 Taunt
    const VANILLA: &str = "core-008"; // 4/4
    const HIT_JOB: &str = "core-016"; // Spell: destroy target Unit
    const SORCERER: &str = "core-068"; // Unit: Cry deal 4 damage to a target
    const SWITCH_ALL: &str = "core-048"; // Spell: switch the position of every Unit
    const NETHER: &str = "core-088"; // Spell: destroy all permanents
    const BIG_FELINOR: &str = "core-043"; // Unit: Cry destroy all non-Felinor Units
    const WHIRLWIND: &str = "classicplus-021";
    const AURA: &str = "core-046"; // Field Spell: "Aura: All Units have −1/−1."
    const HONEYPOT: &str = "core-060"; // Trap; radiant: fill your board with Rush Tokens, they attack a played Unit
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

    /// p2's Top Loser in lane 3; p1 is active with the given side.
    fn top_loser(p1: Value, radiant_face: bool, p2: Value) -> Scenario {
        crate::register_all();
        let side1 = spread(json!({ "hand": [FILLER], "library": DECK }), &p1);
        let mut side2 = spread(json!({ "hand": [FILLER], "library": DECK }), &p2);
        let mut field = vec![json!({ "def": TOP, "lane": 3, "radiant": radiant_face })];
        field.extend(p2.get("field").and_then(Value::as_array).cloned().unwrap_or_default());
        side2["field"] = Value::Array(field);
        scenario(json!({ "p1": side1, "p2": side2 }))
    }

    fn top(s: &Scenario) -> CardInstance {
        match s.unit(P2, 3) {
            Some(unit) if unit.def_id == TOP => unit,
            _ => panic!("no Top Loser in lane 3"),
        }
    }

    /// TS wrote through the live instance `top(s)` handed back.
    fn top_mut(s: &mut Scenario) -> &mut CardInstance {
        let id = top(s).id;
        find_instance_mut(s.state_mut(), &id).expect("no Top Loser in lane 3")
    }

    fn attack_targets(s: &Scenario, attacker_id: &str) -> Vec<String> {
        legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Attack { attacker_id: by, target_id } if by == attacker_id => Some(target_id),
                _ => None,
            })
            .collect()
    }

    use crate::unit_or_blank;

    /// Every `summoned` event for `player`: its instance id and lane.
    fn summoned_for(s: &Scenario, player: PlayerId) -> Vec<(String, i32)> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { player: who, instance_id, lane, .. } if *who == player => {
                    Some((instance_id.clone(), *lane))
                }
                _ => None,
            })
            .collect()
    }

    fn attackers(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AttackDeclared { attacker_id, .. } => Some(attacker_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod c_n19_1_top_loser {
        use super::*;

        #[test]
        fn is_a_2_5_5_unit_token_with_a_printed_rarity_one_script_serves_both_faces() {
            let def = crate::card_def(ID);
            assert!(def.token);
            assert_eq!(def.rarity, Rarity::Token);
            assert_eq!(def.printed_rarity, Some(PrintedRarity::Legendary));
            let scripts = script();
            assert_eq!(
                scripts.base.static_flags,
                Some(StaticFlags {
                    attacked_only_from_lane: Some(true),
                    never_defense: Some(true),
                    ..StaticFlags::default()
                })
            );
            // TS `expect(radiant).toBe(base)`: the radiant face is the base script.
            assert_eq!(scripts.radiant.static_flags, scripts.base.static_flags);
        }

        mod base {
            use super::*;

            #[test]
            fn has_armor_3_a_4_damage_hit_leaves_it_at_4() {
                let mut s = top_loser(json!({ "field": [{ "def": VANILLA, "lane": 3 }] }), false, json!({}));
                assert_eq!(s.stats(top(&s)).armor, 3);
                let attacker = unit_or_blank(&s, P1, 3);
                let target = top(&s);
                s.attack(attacker, &target);
                let target = top(&s);
                s.expect_stats(&target, json!({ "health": 4 }));
            }

            #[test]
            fn r386_its_armor_is_tuned_as_a_numbered_keyword_an_upgrade_makes_it_armor_4() {
                let mut s = top_loser(json!({ "field": [{ "def": VANILLA, "lane": 3 }] }), false, json!({}));
                let tuning = tuning_of(top_mut(&mut s));
                tuning.x = Some(add_step(tuning.x.as_ref(), "Armor", 1));
                assert_eq!(s.stats(top(&s)).armor, 4);
                let attacker = unit_or_blank(&s, P1, 3);
                let target = top(&s);
                s.attack(attacker, &target);
                let target = top(&s);
                s.expect_stats(&target, json!({ "health": 5 }));
            }

            #[test]
            fn s4_1_it_can_t_be_switched_to_defense_position_the_switch_is_refused() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": TOP, "lane": 1 }] },
                    "p2": { "hand": [FILLER] },
                }));
                let unit = unit_or_blank(&s, P1, 1);
                s.expect_refused_with(|s| s.switch_position(&unit), "Defense");
                assert_eq!(s.stats(unit_or_blank(&s, P1, 1)).position, Position::Atk);
            }

            #[test]
            fn s4_1_a_switch_all_effect_leaves_it_in_attack_position() {
                let mut s = top_loser(
                    json!({ "hand": [SWITCH_ALL, FILLER], "field": [{ "def": VANILLA, "lane": 1 }] }),
                    false,
                    json!({}),
                );
                s.play(SWITCH_ALL, json!({}));
                assert_eq!(s.stats(top(&s)).position, Position::Atk);
                assert_eq!(s.stats(unit_or_blank(&s, P1, 1)).position, Position::Def);
            }

            #[test]
            fn s4_2_step_2_only_the_enemy_unit_in_its_own_lane_may_declare_an_attack_on_it() {
                let mut s = top_loser(
                    json!({ "field": [{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 3 }] }),
                    false,
                    json!({}),
                );
                let across = s.unit(P1, 3).expect("setup");
                let beside = s.unit(P1, 2).expect("setup");
                let loser = top(&s);
                assert!(!attack_targets(&s, &beside.id).contains(&loser.id));
                assert!(attack_targets(&s, &across.id).contains(&loser.id));
                s.expect_refused_with(|s| s.attack(&beside, &loser), "lane");
                s.attack(&across, &loser);
                let loser = top(&s);
                s.expect_stats(&loser, json!({ "health": 4 }));
            }

            #[test]
            fn r53_a_forced_attack_from_another_lane_does_not_happen_a_radiant_honeypot_s_tokens_attack_it_only_from_its_lane(
            ) {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": {
                        "hand": [FILLER],
                        "backrow": [{ "def": HONEYPOT, "lane": 1, "faceUp": false, "radiant": true }],
                        "library": DECK,
                    },
                    "p2": { "hand": [TOP, FILLER], "library": DECK },
                }));
                s.play(TOP, json!({ "zone": 3 }));
                // The board filled with five Rush Tokens; only the one in lane 3 attacked (and died to the strike back).
                let tokens = summoned_for(&s, P1);
                assert_eq!(tokens.iter().map(|(_, lane)| *lane).collect::<Vec<i32>>(), vec![1, 2, 3, 4, 5]);
                assert_eq!(attackers(&s), vec![tokens[2].0.clone()]);
            }

            #[test]
            fn s4_2_step_2_a_random_enemy_forced_attack_from_another_lane_never_draws_it() {
                crate::register_all();
                let mut made = 0;
                for seed in 1..=10 {
                    let mut s = scenario(json!({
                        "seed": format!("top-random-{seed}"),
                        "p1": { "hand": [FILLER], "field": [{ "def": JUNGLE, "lane": 1, "radiant": true }], "library": DECK },
                        "p2": {
                            "hand": [FILLER],
                            "field": [{ "def": TOP, "lane": 3 }, { "def": BODY, "lane": 5 }],
                            "library": DECK,
                        },
                    }));
                    let loser = top(&s);
                    s.end_turn();
                    let attacks: Vec<String> = s
                        .events()
                        .iter()
                        .filter_map(|event| match event {
                            GameEvent::AttackDeclared { target_id, .. } => Some(target_id.clone()),
                            _ => None,
                        })
                        .collect();
                    for target in &attacks {
                        assert_ne!(target, &loser.id);
                    }
                    made += attacks.len();
                }
                // The roll came up on some of the seeds, so a forced attack was drawn, and never on it.
                assert!(made > 0);
            }

            #[test]
            fn s4_2_step_3_a_taunt_on_it_binds_only_the_attacker_in_its_lane() {
                let mut s = top_loser(
                    json!({ "field": [{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 3 }] }),
                    false,
                    json!({}),
                );
                top_mut(&mut s).granted_keywords.push(Keyword::Taunt);
                let beside = s.unit(P1, 2).expect("setup");
                let across = s.unit(P1, 3).expect("setup");
                // The lane-2 attacker cannot reach the Taunt, so it binds nothing: the hero is open to it.
                assert!(attack_targets(&s, &beside.id).contains(&"hero-p2".to_string()));
                // The lane-3 attacker can, so it must attack the Taunt.
                assert_eq!(attack_targets(&s, &across.id), vec![top(&s).id]);
            }

            #[test]
            fn effects_still_target_it_a_spell_destroys_it_and_a_token_it_ceases_to_exist() {
                let mut s = top_loser(json!({ "hand": [HIT_JOB, FILLER] }), false, json!({}));
                let loser = top(&s);
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": loser.id }] }));
                s.expect_in_zone(&loser, "gone");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_10_10_with_armor_6_and_immune_to_spells() {
                let mut s = top_loser(json!({}), true, json!({}));
                let loser = top(&s);
                s.expect_stats(&loser, json!({ "attack": 10, "health": 10 }));
                assert_eq!(s.stats(top(&s)).armor, 6);
                assert!(
                    s.stats(top(&s))
                        .keywords
                        .iter()
                        .any(|keyword| keyword.kind() == KeywordKind::ImmuneToSpells)
                );
            }

            #[test]
            fn s6_1_no_spell_targets_it() {
                let mut s = top_loser(
                    json!({ "hand": [HIT_JOB, FILLER] }),
                    true,
                    json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
                );
                let loser = top(&s);
                s.expect_refused(|s| {
                    s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": loser.id }] }))
                });
                let hit_job = s.card(HIT_JOB).id.clone();
                let offered: Vec<Selection> = legal_actions(s.state(), P1)
                    .into_iter()
                    .flat_map(|action| match action {
                        ActionBody::Play { instance_id, targets, .. } if instance_id == hit_job => {
                            targets.unwrap_or_default()
                        }
                        _ => Vec::new(),
                    })
                    .collect();
                assert!(!serde_json::to_string(&offered).expect("JSON").contains(&top(&s).id));
            }

            #[test]
            fn s6_1_no_spell_s_effect_touches_it_whirlwind_and_twisting_nether_pass_it_by() {
                let mut s = top_loser(
                    json!({ "hand": [WHIRLWIND, NETHER, FILLER], "mana": 5 }),
                    true,
                    json!({ "field": [{ "def": BODY, "lane": 1 }] }),
                );
                s.play(WHIRLWIND, json!({}));
                let loser = top(&s);
                s.expect_stats(&loser, json!({ "health": 10 }));
                s.play(NETHER, json!({}));
                assert_eq!(s.unit(P2, 3).map(|unit| unit.def_id), Some(TOP.to_string()));
                assert!(s.unit(P2, 1).is_none());
            }

            #[test]
            fn s6_1_a_unit_still_reaches_it_a_cry_targets_it_and_a_unit_s_sweep_destroys_it() {
                // Radiant Twisted Sorcerer's 8 through the Radiant Top Loser's Armor 6 leaves 2.
                let mut s = top_loser(json!({ "hand": [{ "def": SORCERER, "radiant": true }, FILLER] }), true, json!({}));
                let loser = top(&s);
                s.play(
                    SORCERER,
                    json!({ "zone": 1, "targets": [{ "pick": "instance", "instanceId": loser.id }] }),
                );
                let loser = top(&s);
                s.expect_stats(&loser, json!({ "health": 8 }));
                let mut felinor = top_loser(json!({ "hand": [BIG_FELINOR, FILLER] }), true, json!({}));
                let loser = top(&felinor);
                felinor.play(BIG_FELINOR, json!({ "zone": 1 }));
                felinor.expect_in_zone(&loser, "gone");
            }

            #[test]
            fn s4_2_it_may_still_be_attacked_from_its_lane() {
                let mut s = top_loser(json!({ "field": [{ "def": BODY, "lane": 3 }] }), true, json!({}));
                let attacker = unit_or_blank(&s, P1, 3);
                let target = top(&s);
                s.attack(attacker, &target);
                let target = top(&s);
                s.expect_stats(&target, json!({ "health": 7 }));
            }

            #[test]
            fn s4_1_it_still_can_t_be_in_defense_position() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": TOP, "lane": 1, "radiant": true }] },
                    "p2": { "hand": [FILLER] },
                }));
                let unit = unit_or_blank(&s, P1, 1);
                s.expect_refused_with(|s| s.switch_position(&unit), "Defense");
            }

            #[test]
            fn s6_1_field_spells_and_traps_still_reach_it_suppressive_aura_shrinks_it_a_radiant_honeypot_s_token_attacks_it(
            ) {
                let mut aura = top_loser(json!({ "backrow": [{ "def": AURA, "lane": 1 }] }), true, json!({}));
                let loser = top(&aura);
                aura.expect_stats(&loser, json!({ "attack": 9, "health": 9 }));

                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": {
                        "hand": [FILLER],
                        "backrow": [{ "def": HONEYPOT, "lane": 1, "faceUp": false, "radiant": true }],
                        "library": DECK,
                    },
                    "p2": { "hand": [{ "def": TOP, "radiant": true }, FILLER], "library": DECK },
                }));
                s.play(TOP, json!({ "zone": 3 }));
                let tokens: Vec<String> = summoned_for(&s, P1).into_iter().map(|(id, _)| id).collect();
                assert_eq!(attackers(&s), vec![tokens[2].clone()]);
            }
        }
    }
}
