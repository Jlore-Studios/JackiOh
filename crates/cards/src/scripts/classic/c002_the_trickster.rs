//! C #2 The Trickster (SPEC §8.6 row 2, §6.3 Cost; R65, R70). Unit, Human, cost 1, Common,
//! 2/1 → 4/2.
//!   Base:    "Cry: Your next Trap or Field Spell costs ({discount}) less."
//!   Radiant: "Cry: Your next Trap or Field Spell costs (0)."
//!   Engine:  "A player modifier like #35 Lunar Eclipse's next-Spell discount (`costDiscount`; Cost,
//!            §6.3, R65) for Traps, Field Traps and Field Spells, consumed by the next such card you
//!            play; "next" has no "this turn", so it waits across turns until used; Field Trap counts
//!            as Trap. The Radiant face sets that card's cost to (0) instead of discounting it. Tunes:
//!            discount 2 ↑."
//!
//! THE MODIFIER is a price rule on its controller (B5 E15, `addCostRule`) that lasts "until used": it
//! reaches Traps, Field Traps and Field Spells ("Trap" names "Field Trap" too), and the first play of
//! one whose price it changed spends it (`mana.costRulesSpentBy`), whatever turn that is. A Spell or
//! a Unit is never reached, so it neither uses nor spends it. R65 floors the price at (0). A cast pays
//! nothing, so it never uses the rule and never spends it (R70).
//!
//! The base face's discount is the card's declared number (`param(ctx, "discount")`), a negative
//! `amount`; the Radiant face's "costs (0)" is a `setTo`, which wins over every add (B5 E15).

use jackioh_engine::effects::add_cost_rule;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-002";

/// "Trap or Field Spell": a Field Trap is a Trap.
const REACHES: &[CardType] = &[CardType::Trap, CardType::FieldTrap, CardType::FieldSpell];

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![add_cost_rule(json_as(json!({
                "rule": { "types": REACHES, "amount": -param(&*ctx, "discount") },
                "lasts": "used",
            })))]
        })),
        ..Script::default()
    };

    let radiant = Script {
        // "costs (0)": the declared number `setCost` (R386).
        cry: Some(hook(|ctx| {
            vec![add_cost_rule(json_as(json!({
                "rule": { "types": REACHES, "setTo": param(&*ctx, "setCost") },
                "lasts": "used",
            })))]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #2 The Trickster — SPEC §8.6 row 2, BUILD M9 Classic row C 2: "Cry: your next Trap, Field Trap
// or Field Spell costs (2) less, a player modifier with no "this turn" that waits across turns until
// the first such play consumes it; Spells and Units neither use nor consume it; floors at (0) (R65); a
// cast never uses it (R70); the opponent's view of your changed hand costs shows −1 (R177); radiant
// 4/2: the next one costs (0); its tuned number (discount) reads through `param()` (R386)".
//
// The cast case uses Classic+ #37 Wardrum, whose end-of-turn copy is a cast Trap.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TRICKSTER: &str = "classic-002";
    const EXPERIMENT: &str = "core-085"; // (2) Trap
    const SHEEPISH: &str = "core-041"; // (1) Trap
    const TESLA: &str = "classic-005"; // (2) Field Trap
    const MANA_WELL: &str = "core-006"; // (3) Field Spell
    const STOCKPILE: &str = "core-005"; // (1) Spell
    const VANILLA: &str = "core-008"; // (1) Unit
    const WARDRUM: &str = "classicplus-037"; // (5) Unit: "End of turn: Cast a copy of a random Spell, Field Spell or Trap you played this turn."

    use crate::js;

    fn cost_of(s: &Scenario, card: &str) -> i32 {
        effective_cost(s.state(), s.card(card), Default::default())
    }

    /// `"you" | "opponent"`: the side of the viewer's view whose modifiers are read.
    fn modifier_labels(s: &Scenario, viewer: PlayerId, side: &str) -> Vec<String> {
        let view = js(&s.view(viewer));
        view[side]["modifiers"]
            .as_array()
            .map(|mods| mods.iter().filter_map(|m| m["label"].as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    }

    mod c_n2_the_trickster {
        use super::*;

        #[test]
        fn declares_its_two_numbers_discount_and_the_radiant_set_cost_r386() {
            crate::register_all();
            assert_eq!(
                js(&crate::card_def(TRICKSTER).params),
                json!([
                    { "key": "discount", "base": 2, "radiant": 2, "better": "up", "step": 1, "min": 1, "tunedOn": "base" },
                    { "key": "setCost", "base": 0, "radiant": 0, "better": "down", "step": 1, "min": 0 }
                ])
            );
            let scripts = script();
            assert!(scripts.base.targets.is_empty());
            assert!(scripts.radiant.targets.is_empty());
        }

        mod base {
            use super::*;

            #[test]
            fn is_a_2_1() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [STOCKPILE], "field": [TRICKSTER] }, "p2": { "hand": [STOCKPILE] } }));
                s.expect_stats(TRICKSTER, json!({ "attack": 2, "health": 1, "maxHealth": 1 }));
            }

            #[test]
            fn cry_your_next_trap_costs_2_less() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [TRICKSTER, EXPERIMENT, STOCKPILE] }, "p2": { "hand": [STOCKPILE] } }));
                assert_eq!(cost_of(&s, EXPERIMENT), 2);
                s.play(TRICKSTER, json!({}));
                assert_eq!(cost_of(&s, EXPERIMENT), 0);
                s.play(EXPERIMENT, json!({ "zone": 1 }));
                // 4 − 1 (Trickster) − 0.
                s.expect_mana(P1, 3);
            }

            #[test]
            fn a_field_trap_counts_as_a_trap_and_a_field_spell_is_reached_too() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [TRICKSTER, TESLA, MANA_WELL, STOCKPILE] }, "p2": { "hand": [STOCKPILE] } }));
                s.play(TRICKSTER, json!({}));
                assert_eq!(cost_of(&s, TESLA), 0);
                assert_eq!(cost_of(&s, MANA_WELL), 1);
            }

            #[test]
            fn the_first_such_play_consumes_it_the_next_one_pays_full_price() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [TRICKSTER, MANA_WELL, SHEEPISH, STOCKPILE] }, "p2": { "hand": [STOCKPILE] } }));
                s.play(TRICKSTER, json!({}));
                s.play(MANA_WELL, json!({ "zone": 1 }));
                s.expect_mana(P1, 2); // 4 − 1 − 1
                assert_eq!(cost_of(&s, SHEEPISH), 1);
                assert_eq!(modifier_labels(&s, P1, "you"), Vec::<String>::new());
            }

            #[test]
            fn the_first_such_play_consumes_it_even_when_that_card_already_costs_0_under_cloaked_toe_cracker() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TRICKSTER, EXPERIMENT, MANA_WELL, STOCKPILE], "field": ["classic-006"] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                s.play(TRICKSTER, json!({}));
                assert_eq!(cost_of(&s, EXPERIMENT), 0);
                s.play(EXPERIMENT, json!({ "zone": 1 }));
                assert_eq!(modifier_labels(&s, P1, "you"), Vec::<String>::new());
                assert_eq!(cost_of(&s, MANA_WELL), 3);
            }

            #[test]
            fn r65_the_price_floors_at_0_a_1_cost_trap_costs_0_not_less() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [TRICKSTER, SHEEPISH, STOCKPILE] }, "p2": { "hand": [STOCKPILE] } }));
                s.play(TRICKSTER, json!({}));
                assert_eq!(cost_of(&s, SHEEPISH), 0);
                s.play(SHEEPISH, json!({ "zone": 1 }));
                s.expect_mana(P1, 3);
            }

            #[test]
            fn spells_and_units_neither_use_nor_consume_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TRICKSTER, STOCKPILE, VANILLA, EXPERIMENT], "library": [STOCKPILE, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                s.play(TRICKSTER, json!({}));
                assert_eq!(cost_of(&s, STOCKPILE), 1);
                assert_eq!(cost_of(&s, VANILLA), 1);
                s.play(STOCKPILE, json!({}));
                s.play(VANILLA, json!({}));
                assert_eq!(cost_of(&s, EXPERIMENT), 0);
                assert_eq!(modifier_labels(&s, P1, "you"), vec!["Your next Trap or Field Spell costs (2) less"]);
            }

            #[test]
            fn no_this_turn_it_waits_across_turns_until_used() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TRICKSTER, MANA_WELL, STOCKPILE], "library": [STOCKPILE, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE, STOCKPILE], "library": [STOCKPILE, STOCKPILE] },
                }));
                s.play(TRICKSTER, json!({}));
                s.end_turn();
                assert_eq!(s.state().active, P2);
                s.end_turn();
                assert_eq!(s.state().active, P1);
                assert_eq!(cost_of(&s, MANA_WELL), 1);
                s.play(MANA_WELL, json!({ "zone": 1 }));
                assert_eq!(modifier_labels(&s, P1, "you"), Vec::<String>::new());
            }

            #[test]
            fn it_is_yours_the_opponent_s_traps_are_untouched() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [TRICKSTER, STOCKPILE] }, "p2": { "hand": [EXPERIMENT, STOCKPILE] } }));
                s.play(TRICKSTER, json!({}));
                assert_eq!(cost_of(&s, EXPERIMENT), 2);
            }

            #[test]
            fn r177_the_opponent_reads_your_hand_as_a_count_and_no_cost_of_it_only_the_public_badge() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [TRICKSTER, EXPERIMENT, STOCKPILE] }, "p2": { "hand": [STOCKPILE] } }));
                s.play(TRICKSTER, json!({}));
                let theirs = js(&s.view(P2));
                assert_eq!(theirs["opponent"]["hand"], json!({ "count": 2 }));
                for event in theirs["events"].as_array().cloned().unwrap_or_default() {
                    if event["type"] == "costChanged" {
                        assert_eq!(event["cost"], json!(-1));
                    }
                }
                assert!(!theirs.to_string().contains(EXPERIMENT));
                assert_eq!(modifier_labels(&s, P2, "opponent"), vec!["Your next Trap or Field Spell costs (2) less"]);
                // The change is real in your own view: the Trap reads (0) there.
                let your_hand = js(&s.view(P1))["you"]["hand"].clone();
                let Some(cards) = your_hand.as_array() else {
                    panic!("your own hand travels in full (§10.8)");
                };
                let experiment = cards.iter().find(|card| card["defId"] == EXPERIMENT);
                assert_eq!(experiment.map(|card| card["cost"].clone()), Some(json!(0)));
            }

            #[test]
            fn r70_a_cast_never_uses_it_a_trap_wardrum_casts_leaves_the_discount_waiting() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [SHEEPISH, TRICKSTER, MANA_WELL, STOCKPILE], "field": [WARDRUM], "library": [STOCKPILE, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE, STOCKPILE], "library": [STOCKPILE, STOCKPILE] },
                }));
                s.play(SHEEPISH, json!({ "zone": 1 })); // before the Trickster: full price
                s.play(TRICKSTER, json!({}));
                s.end_turn(); // Wardrum casts a copy of the Sheepish
                // The played Sheepish and the cast copy: two plays of it, the second paying nothing (R70).
                let plays = s
                    .events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == SHEEPISH))
                    .count();
                assert_eq!(plays, 2);
                assert_eq!(modifier_labels(&s, P1, "you"), vec!["Your next Trap or Field Spell costs (2) less"]);
            }

            #[test]
            fn r386_an_upgrade_of_discount_makes_it_3_less() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [TRICKSTER, MANA_WELL, STOCKPILE] }, "p2": { "hand": [STOCKPILE] } }));
                step_param(s.card_mut(TRICKSTER), "discount", 1);
                s.play(TRICKSTER, json!({}));
                assert_eq!(cost_of(&s, MANA_WELL), 0);
                assert_eq!(modifier_labels(&s, P1, "you"), vec!["Your next Trap or Field Spell costs (3) less"]);
            }

            #[test]
            fn r386_a_degrade_of_discount_makes_it_1_less() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [TRICKSTER, MANA_WELL, STOCKPILE] }, "p2": { "hand": [STOCKPILE] } }));
                step_param(s.card_mut(TRICKSTER), "discount", -1);
                s.play(TRICKSTER, json!({}));
                assert_eq!(cost_of(&s, MANA_WELL), 2);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_a_4_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [STOCKPILE], "field": [{ "def": TRICKSTER, "radiant": true }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                s.expect_stats(TRICKSTER, json!({ "attack": 4, "health": 2, "maxHealth": 2 }));
            }

            #[test]
            fn cry_your_next_trap_field_trap_or_field_spell_costs_0() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TRICKSTER, "radiant": true }, MANA_WELL, TESLA, EXPERIMENT, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                s.play(TRICKSTER, json!({}));
                assert_eq!(cost_of(&s, MANA_WELL), 0);
                assert_eq!(cost_of(&s, TESLA), 0);
                assert_eq!(cost_of(&s, EXPERIMENT), 0);
                assert_eq!(modifier_labels(&s, P1, "you"), vec!["Your next Trap or Field Spell costs (0)"]);
                s.play(MANA_WELL, json!({ "zone": 1 }));
                s.expect_mana(P1, 3);
                // Consumed by that play.
                assert_eq!(cost_of(&s, EXPERIMENT), 2);
            }

            #[test]
            fn r386_a_degrade_makes_the_next_one_cost_1_and_an_upgrade_finds_the_cost_at_its_floor_of_0() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TRICKSTER, "radiant": true }, MANA_WELL, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                assert!(!crate::can_upgrade_number(&s, TRICKSTER, "setCost"));
                assert_eq!(crate::degrade_number(&mut s, TRICKSTER, "setCost"), 1);
                s.play(TRICKSTER, json!({}));
                assert_eq!(cost_of(&s, MANA_WELL), 1);
            }

            #[test]
            fn spells_and_units_neither_use_nor_consume_it_and_it_waits_across_turns() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TRICKSTER, "radiant": true }, STOCKPILE, MANA_WELL], "library": [STOCKPILE, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE, STOCKPILE], "library": [STOCKPILE, STOCKPILE] },
                }));
                s.play(TRICKSTER, json!({}));
                assert_eq!(cost_of(&s, STOCKPILE), 1);
                s.play(STOCKPILE, json!({}));
                s.end_turn();
                s.end_turn();
                assert_eq!(cost_of(&s, MANA_WELL), 0);
            }
        }
    }
}
