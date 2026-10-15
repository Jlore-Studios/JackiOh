//! #44 True Strike (SPEC §8.2, R346). Spell, cost 1, Common.
//!   Base:    "Pierce / Deal 4 damage. Exile this."
//!   Radiant: "Pierce / Deal 9 damage. Exile this." (the faces differ only in the number.)
//!   Engine:  "Pierce: skips pipeline step 2; Divine Shield still applies."
//!
//! PIERCE ON A SPELL (R346): its damage ignores Armor. §4.4 step 2 reads Pierce off the source
//! (`damage.pierces`), and the effect states it too (`ignoreArmor`, the effect's own Pierce as R85's
//! `lifesteal` is Lifesteal), so the hit pierces on every run, an Echo repeat included, whose source R98
//! may have left empty once "exile this" took the card from the resolving zone. It skips step 2 and
//! NOTHING else: step 1 still negates a hit on a Divine Shield, step 3 still clamps Anti-oneshot Armor,
//! step 4 still makes an Indestructible target take nothing.
//! "Deal 4 damage" takes a DECLARED play-time target (R81, §8 Conventions: any unit or hero), read as
//! `ctx.targets[0]`, never a prompt; R90 validates it against the declaration.
//! "Exile this" (§5.1, §6.3): the spell is in the `resolving` zone while its script runs (§10.1, §10.5
//! step 4), so `exile` moves it from there and step 7 must not send it to the graveyard. `exile` bumps
//! the exile counter (R55), which #40 Echoes of the Forgotten and #100 Ceaseless Void read.
//! Damage then exile in one list: the state check runs after both (R59).

use jackioh_engine::effects::{damage, exile};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-044";

/// The faces differ only in the declared number `damage` (R386): 4 and 9, read off the running face.
fn true_strike() -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))],
        // A Spell's script hangs off `cry`: that is its on-resolve hook (§10.9). R346: the hit pierces.
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "damage");
            vec![
                damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount, "ignoreArmor": true }))),
                exile(json_as(json!({ "target": { "of": "self" } }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = true_strike();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #44 True Strike (SPEC §8.2, §4.4, §5.1, §6.3 Exile; R63, R65, R81, R90, R346).
// Must-pass row (BUILD M4-T4 #44): "4 damage through Armor 7; Divine Shield still blocks; exiled;
// radiant 9." Pierce skips §4.4 step 2 only, so each half is one test: #25 4-mana 7/7 (Armor 7) takes
// the full hit, #56 Jilliax (Divine Shield) takes none because step 1 negates the whole instance.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::damage::{DamageArgs, DamageTarget, deal_damage};
    use jackioh_engine::layers::unit_view;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    /// The catalog card this file scripts.
    fn def() -> CardDef {
        crate::register_all();
        crate::card_def(ID)
    }

    /// The declared play-time pick (R81): a `Selection` naming the enemy unit in lane `lane`.
    fn at_unit(s: &Scenario, lane: i32) -> Value {
        let Some(target) = s.unit(PlayerId::P2, lane) else {
            panic!("no p2 unit in lane {lane}");
        };
        json!({ "targets": [{ "pick": "instance", "instanceId": target.id }] })
    }

    /// p1 holds True Strike; #21 Hinder rides along so the turn does not auto-end (R82). `enemy` is
    /// the field entry `{ def, radiant? }` put in p2's lane 1.
    fn spell(enemy: Value, radiant_spell: bool) -> Scenario {
        let mut entry = enemy;
        entry["lane"] = json!(1);
        scenario(json!({
            "seed": "true-strike",
            "p1": { "hand": [{ "def": "core-044", "radiant": radiant_spell }, "core-021"] },
            "p2": { "field": [entry], "health": 30 },
        }))
    }

    fn graveyard_holds_true_strike(s: &Scenario) -> bool {
        s.pile(PlayerId::P1, "graveyard").iter().any(|card| card.def_id == "core-044")
    }

    mod card_data_r346 {
        use super::*;

        #[test]
        fn r346_prints_pierce_on_both_faces_with_the_patchs_text() {
            let def = def();
            assert_eq!(serde_json::to_value(&def.base.keywords).unwrap(), json!([{ "kind": "Pierce" }]));
            assert_eq!(serde_json::to_value(&def.radiant.keywords).unwrap(), json!([{ "kind": "Pierce" }]));
            // The face as printed: its declared `damage` filled in (R386, R482).
            assert_eq!(fill_params(&def, FaceKind::Base, None), "Pierce\nDeal 4 damage. Exile this.");
            assert_eq!(fill_params(&def, FaceKind::Radiant, None), "Pierce\nDeal 9 damage. Exile this.");
        }

        #[test]
        fn r346_the_printed_keyword_is_a_pierce_of_its_own_the_card_as_a_source_skips_step_2_without_the_flag() {
            // The pipeline reads Pierce off the source (§10.4's reading of any card), so a True Strike
            // that is the source of a hit carrying no `ignoreArmor` still puts its whole amount through.
            let s = scenario(json!({
                "seed": "true-strike",
                "p1": { "hand": ["core-044"] },
                "p2": { "field": ["core-025"] },
            }));
            let strike = s.card("core-044").clone();
            let keywords = serde_json::to_value(&unit_view(s.state(), &strike).keywords).unwrap();
            assert!(keywords.as_array().unwrap().contains(&json!({ "kind": "Pierce" })));
            let Some(wall) = s.unit(PlayerId::P2, 1) else {
                panic!("no wall");
            };
            // A copy of the state, since nothing reads it afterwards.
            let mut state = s.state().clone();
            let mut events = Vec::new();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let dealt = deal_damage(
                &mut sink,
                DamageArgs {
                    source: Some(strike),
                    target: DamageTarget::Unit { instance: wall },
                    amount: 4,
                    flags: Default::default(),
                },
            );
            assert_eq!(dealt, 4);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn deals_4_damage_through_armor_7_skipping_step_2_and_only_step_2() {
            let mut s = spell(json!({ "def": "core-025" }), false);
            let pick = at_unit(&s, 1);
            s.play("core-044", pick);

            // 7 health − 4 = 3. With Armor applied the hit would have been 0 and nothing would show.
            s.expect_stats("core-025", json!({ "attack": 7, "health": 3, "maxHealth": 7 }));
            s.expect_events(json!(["cardPlayed", "damage"]));
        }

        #[test]
        fn r63_divine_shield_still_blocks_the_whole_hit_and_is_spent() {
            let mut s = spell(json!({ "def": "core-056" }), false);
            let pick = at_unit(&s, 1);
            s.play("core-044", pick);

            s.expect_stats("core-056", json!({ "attack": 3, "health": 2, "maxHealth": 2 }));
            s.expect_in_zone("core-056", "field");
            assert_eq!(s.unit(PlayerId::P2, 1).and_then(|card| card.divine_shield_spent), Some(true));
            s.expect_events(json!(["cardPlayed", "divineShieldLost"]));
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Damage));
        }

        #[test]
        fn s5_1_and_s6_3_exile_this_sends_the_spell_to_exile_not_to_the_graveyard() {
            let mut s = spell(json!({ "def": "core-025" }), false);
            let pick = at_unit(&s, 1);
            s.play("core-044", pick);

            s.expect_in_zone("core-044", "exile");
            s.expect_events(json!(["exiled"]));
            assert!(!graveyard_holds_true_strike(&s));
        }

        #[test]
        fn s8_conventions_a_target_includes_a_hero_on_either_side() {
            let mut s = spell(json!({ "def": "core-025" }), false);
            s.play("core-044", json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            s.expect_health(PlayerId::P2, 26);
        }

        #[test]
        fn r346_ignores_a_heros_armor_too() {
            let mut s = scenario(json!({
                "seed": "true-strike",
                "p1": { "hand": ["core-044", "core-021"] },
                "p2": { "health": 30, "armor": 5 },
            }));
            s.play("core-044", json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            // With Armor applied, 4 − 5 would floor at 0 and the hero would still be at 30.
            s.expect_health(PlayerId::P2, 26);
        }

        #[test]
        fn r81_declares_exactly_one_target_any_side_unit_or_hero_as_a_play_time_choice() {
            let base = script().base;
            assert_eq!(
                serde_json::to_value(&base.targets).unwrap(),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "hero"] } }]),
            );
            assert!(base.modes.is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r346_deals_9_damage_with_pierce_only_the_number_changed() {
            // Radiant #43 Big Felinor is a 6/20 with no Armor, so the damage dealt is visible exactly.
            let mut s = spell(json!({ "def": "core-043", "radiant": true }), true);
            let pick = at_unit(&s, 1);
            s.play("core-044", pick);

            s.expect_stats("core-043", json!({ "attack": 6, "health": 11, "maxHealth": 20 }));
        }

        #[test]
        fn nine_through_armor_7_kills_the_4_mana_7_7_outright() {
            let mut s = spell(json!({ "def": "core-025" }), true);
            let pick = at_unit(&s, 1);
            s.play("core-044", pick);

            s.expect_in_zone("core-025", "graveyard");
            s.expect_events(json!(["cardPlayed", "damage", "destroyed"]));
        }

        #[test]
        fn r63_divine_shield_still_blocks_all_9() {
            let mut s = spell(json!({ "def": "core-056" }), true);
            let pick = at_unit(&s, 1);
            s.play("core-044", pick);

            s.expect_stats("core-056", json!({ "attack": 3, "health": 2, "maxHealth": 2 }));
            s.expect_in_zone("core-056", "field");
            assert_eq!(s.unit(PlayerId::P2, 1).and_then(|card| card.divine_shield_spent), Some(true));
        }

        #[test]
        fn still_exiles_itself_the_unrestated_clause_is_kept() {
            let mut s = spell(json!({ "def": "core-025" }), true);
            let pick = at_unit(&s, 1);
            s.play("core-044", pick);

            s.expect_in_zone("core-044", "exile");
            assert!(!graveyard_holds_true_strike(&s));
        }

        #[test]
        fn nine_to_a_hero_ignoring_its_armor() {
            let mut s = scenario(json!({
                "seed": "true-strike",
                "p1": { "hand": [{ "def": "core-044", "radiant": true }, "core-021"] },
                "p2": { "health": 30, "armor": 5 },
            }));
            s.play("core-044", json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            s.expect_health(PlayerId::P2, 21);
        }

        #[test]
        fn r81_the_radiant_face_declares_the_same_single_target() {
            let scripts = script();
            assert_eq!(scripts.radiant.targets, scripts.base.targets);
        }
    }

    #[test]
    fn r386_an_upgrade_deals_5_and_a_degrade_3() {
        for (upgrade, amount) in [(true, 5), (false, 3)] {
            let mut s = scenario(json!({
                "seed": "true-strike",
                "p1": { "hand": ["core-044", "core-021"] },
                "p2": { "health": 30, "armor": 5 },
            }));
            let moved = if upgrade {
                crate::upgrade_number(&mut s, "core-044", "damage")
            } else {
                crate::degrade_number(&mut s, "core-044", "damage")
            };
            assert_eq!(moved, amount);
            s.play("core-044", json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            s.expect_health(PlayerId::P2, 30 - amount);
        }
    }
}
