//! C #35 Prep (SPEC §8.6 row 35). (0) Spell, Common.
//!   Base:    "Your next Spell this turn costs ({discount}) less." — discount 2
//!   Radiant: "Your next Spell this turn costs ({discount}) less." — discount 4
//!   Engine:  "#35 Lunar Eclipse's modifier (`costDiscount`, `onlyType: "Spell"`, this turn, consumed
//!            on use or at cleanup; Cost, §6.3, R65). Tunes: discount 2 ↑."
//!
//! The discount is Core #35 Lunar Eclipse's player modifier, the same shape:
//!   * `onlyType: "Spell"` — `effectiveCost` skips it for a Unit, a Field Spell or a Trap, so those
//!     plays neither pay less nor spend it;
//!   * `oncePerTurn` — §10.5 step 2 (`playSteps.consumeUsedDiscounts`) removes it on the first Spell
//!     it priced, so only the next Spell is cheaper;
//!   * `{ until: "thisTurn" }` — §2.2's cleanup takes it if no Spell used it.
//!
//! R65 floors the price at (0), so a (1) Spell under a 2 discount costs (0) and gives nothing back.
//! R70: a cast pays nothing and never uses a discount, so a Spell cast this turn leaves it for the
//! next Spell played.
//!
//! The amount is the declared number `discount` (R386), 2 or 4, read through `param`; both faces run
//! this one script.

use jackioh_engine::prelude::*;
use jackioh_engine::effects::add_player_modifier;

pub const ID: &str = "classic-035";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![add_player_modifier(json_as(json!({
                "player": "self",
                "mod": {
                    "kind": "costDiscount",
                    "amount": param(ctx, "discount"),
                    // "your next Spell": Spells only, and only the first one.
                    "onlyType": "Spell",
                    "oncePerTurn": true,
                    // "this turn": §2.2's cleanup takes it if unused.
                    "expiry": { "until": "thisTurn", "turn": ctx.state.turn },
                },
            })))]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face's 4 is its declared `discount`, which `param` reads off the running face.
    CardScripts { radiant: base.clone(), base }
}

// C #35 Prep — SPEC §8.6 row 35, BUILD M9 Classic row C 35: "Your next Spell this turn costs (2) less
// (#35 Lunar Eclipse's modifier: the Spell type only, floored at (0), R65); a Unit, Field Spell or
// Trap play doesn't consume it and the first Spell does; it expires at cleanup; a cast never uses it
// (R70); radiant: (4) less; its tuned number (discount) reads through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::{script, ID};
    use jackioh_engine::testkit::*;

    const PREP: &str = "classic-035";
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const ARMOR: &str = "core-073"; // (2) Field Spell; Cry: Draw 1.
    const SHEEPISH: &str = "core-041"; // (1) Trap.
    const VIRUS: &str = "core-090-1"; // CN-Virus: Cast on draw: take 1 damage.
    const ANCHOR: &str = "core-010"; // (0) Spell, Combo 3 — a free play that keeps a turn open.
    const MENACE: &str = "core-019";

    use crate::js;

    /// Every `cardPlayed` so far, as `{ defId, costPaid }`.
    fn spells_paid(s: &Scenario) -> Vec<Value> {
        s.events()
            .iter()
            .map(js)
            .filter(|event| event["type"] == "cardPlayed")
            .map(|event| json!({ "defId": event["defId"], "costPaid": event["costPaid"] }))
            .collect()
    }

    fn paid_for(s: &Scenario, def_id: &str) -> Vec<i64> {
        spells_paid(s)
            .iter()
            .filter(|play| play["defId"] == def_id)
            .map(|play| play["costPaid"].as_i64().expect("a cardPlayed event's costPaid"))
            .collect()
    }

    mod c_35_prep {
        use super::*;

        #[test]
        fn runs_one_script_on_both_faces() {
            crate::register_all();
            assert_eq!(js(&registered_catalog()[ID])["id"], PREP);
            // TS `expect(radiant).toBe(base)`: the Radiant face is the same script, the same Cry.
            let scripts = script();
            assert!(scripts.base.cry.is_some());
            assert_eq!(scripts.radiant.cry.is_some(), scripts.base.cry.is_some());
            assert_eq!(js(&scripts.radiant.targets), js(&scripts.base.targets));
        }

        mod base {
            use super::*;

            #[test]
            fn r65_the_next_spell_costs_2_less_floored_at_0_a_1_spell_is_free_and_gives_no_mana_back() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PREP, STOCKPILE, ANCHOR], "library": [MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(PREP, json!({}));
                s.play(STOCKPILE, json!({}));

                assert_eq!(paid_for(&s, STOCKPILE), vec![0]);
                s.expect_mana(PlayerId::P1, 4);
            }

            #[test]
            fn the_next_spell_is_the_next_one_whatever_it_costs_a_0_spell_played_next_consumes_it_and_the_spell_after_pays_in_full() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PREP, ANCHOR, STOCKPILE, FLOOD], "library": [MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(PREP, json!({}));
                s.play(ANCHOR, json!({}));
                s.play(STOCKPILE, json!({}));

                assert_eq!(paid_for(&s, ANCHOR), vec![0]);
                assert_eq!(paid_for(&s, STOCKPILE), vec![1]);
                s.expect_mana(PlayerId::P1, 3);
            }

            #[test]
            fn only_the_next_spell_the_first_spell_consumes_it_and_the_second_pays_in_full() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PREP, STOCKPILE, STOCKPILE, ANCHOR], "library": [MENACE, MENACE, MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR] },
                }));
                let (first, second) = {
                    let hand = s.hand(PlayerId::P1);
                    match (hand.get(1), hand.get(2)) {
                        (Some(first), Some(second)) => (first.id.clone(), second.id.clone()),
                        _ => panic!("two Stockpiles in hand"),
                    }
                };

                s.play(PREP, json!({}));
                s.play(&first, json!({}));
                s.play(&second, json!({}));

                assert_eq!(paid_for(&s, STOCKPILE), vec![0, 1]);
                s.expect_mana(PlayerId::P1, 3);
            }

            #[test]
            fn a_unit_play_pays_in_full_and_does_not_consume_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PREP, VANILLA, STOCKPILE, ANCHOR], "library": [MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(PREP, json!({}));
                s.play(VANILLA, json!({}));
                s.play(STOCKPILE, json!({}));

                assert_eq!(paid_for(&s, VANILLA), vec![1]);
                assert_eq!(paid_for(&s, STOCKPILE), vec![0]);
                s.expect_mana(PlayerId::P1, 3);
            }

            #[test]
            fn a_trap_play_pays_in_full_and_does_not_consume_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PREP, SHEEPISH, STOCKPILE, ANCHOR], "library": [MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(PREP, json!({}));
                s.play(SHEEPISH, json!({}));
                s.play(STOCKPILE, json!({}));

                assert_eq!(paid_for(&s, SHEEPISH), vec![1]);
                assert_eq!(paid_for(&s, STOCKPILE), vec![0]);
            }

            #[test]
            fn r70_a_field_spell_play_does_not_consume_it_nor_does_the_spell_its_draw_casts_the_next_spell_played_is_still_cheaper() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PREP, ARMOR, STOCKPILE, ANCHOR], "library": [VIRUS, MENACE, MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(PREP, json!({}));
                s.play(ARMOR, json!({}));

                assert_eq!(paid_for(&s, ARMOR), vec![2]);
                // The Field Spell's Cry drew the CN-Virus, which cast itself for free (R70) and drew again.
                assert_eq!(paid_for(&s, VIRUS), vec![0]);
                s.expect_health(PlayerId::P1, 29);

                s.play(STOCKPILE, json!({}));

                assert_eq!(paid_for(&s, STOCKPILE), vec![0]);
                s.expect_mana(PlayerId::P1, 2);
            }

            #[test]
            fn s2_2_it_expires_at_cleanup_a_spell_on_your_next_turn_pays_in_full() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PREP, STOCKPILE, ANCHOR], "library": [MENACE, MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR], "library": [MENACE, MENACE] },
                }));

                s.play(PREP, json!({}));
                s.end_turn();
                s.end_turn();
                assert_eq!(s.state().active, PlayerId::P1);
                s.play(STOCKPILE, json!({}));

                assert_eq!(paid_for(&s, STOCKPILE), vec![1]);
            }

            #[test]
            fn r386_an_upgrade_makes_it_3_less_a_degrade_1_less_a_4_flood_costs_1_then_3() {
                crate::register_all();
                let mut up = scenario(json!({ "p1": { "hand": [PREP, FLOOD, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));
                step_param(up.card_mut(PREP), "discount", 1);
                up.play(PREP, json!({}));
                up.play(FLOOD, json!({}));
                assert_eq!(paid_for(&up, FLOOD), vec![1]);

                let mut down = scenario(json!({ "p1": { "hand": [PREP, FLOOD, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));
                step_param(down.card_mut(PREP), "discount", -1);
                down.play(PREP, json!({}));
                down.play(FLOOD, json!({}));
                assert_eq!(paid_for(&down, FLOOD), vec![3]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_next_spell_costs_4_less_a_4_flood_is_free() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": PREP, "radiant": true }, FLOOD, ANCHOR] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(PREP, json!({}));
                s.play(FLOOD, json!({}));

                assert_eq!(paid_for(&s, FLOOD), vec![0]);
                s.expect_mana(PlayerId::P1, 4);
            }

            #[test]
            fn still_only_the_next_spell_and_still_not_a_unit() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": PREP, "radiant": true }, VANILLA, FLOOD, STOCKPILE, ANCHOR],
                        "library": [MENACE, MENACE],
                        "mana": 10,
                    },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(PREP, json!({}));
                s.play(VANILLA, json!({}));
                s.play(FLOOD, json!({}));
                s.play(STOCKPILE, json!({}));

                assert_eq!(paid_for(&s, VANILLA), vec![1]);
                assert_eq!(paid_for(&s, FLOOD), vec![0]);
                assert_eq!(paid_for(&s, STOCKPILE), vec![1]);
            }

            #[test]
            fn r386_a_degrade_on_the_radiant_face_steps_4_to_3_a_4_flood_costs_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": PREP, "radiant": true }, FLOOD, ANCHOR] },
                    "p2": { "hand": [ANCHOR] },
                }));
                step_param(s.card_mut(PREP), "discount", -1);

                s.play(PREP, json!({}));
                s.play(FLOOD, json!({}));

                assert_eq!(paid_for(&s, FLOOD), vec![1]);
            }
        }
    }
}
