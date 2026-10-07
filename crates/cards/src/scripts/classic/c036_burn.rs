//! C #36 Burn (SPEC §8.6 row 36). (0) Spell, Common.
//!   Base:    "Deal {damage} damage. If you have {threshold} or more mana left, draw {draw}." — 2, 4, 1
//!   Radiant: "Deal {damage} damage. If your max mana is {threshold} or more, draw {draw}." — 4, 4, 1
//!   Engine:  "Mana left is current mana as it resolves; max mana is §2.3's. The name is also a rules
//!            word, which the reference proof never reads as this card unless `refs` lists it (R381).
//!            Tunes: damage 2 ↑; threshold 4 ↓; draw 1 ↑."
//!
//! "Deal N damage" with no target named is targeted (§8's Conventions, as #68's is): any unit or hero,
//! either side, chosen with the play (R81). One §4.4 hit, then the draw if the condition holds.
//!
//! The condition, read as the Spell resolves: the base face compares the mana its controller has left
//! then (current mana, after paying for Burn, §2.3's temporary mana included); the Radiant face
//! compares §2.3's max mana, which paying does not move. `drawCondition` holds both readings, so the
//! Cry and the glow cannot disagree.
//!
//! R195: in hand the card glows when it would draw if played now — the base face with the mana that
//! would be left after paying its price now (R65's `effectiveCost`, so a discount or a surcharge moves
//! it), the Radiant face with max mana as it stands.
//!
//! The numbers are the declared `damage`, `threshold` and `draw` (R386), read through `param`.

use jackioh_engine::prelude::*;
use jackioh_engine::effects::{damage, draw};

pub const ID: &str = "classic-036";

/// §8 Conventions: any unit or hero, either side.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))]
}

/// The mana the face's condition compares, and whether it reaches the threshold. `leftNow` is the
/// base face's "mana left": current mana as the Spell resolves, or, asked of a card in hand, what
/// paying for it now would leave.
///
/// TS takes `EffectContext | ConditionContext` and reads `state`, `controller` and
/// `param(ctx, "threshold")` off it; the two are different types here, so the caller hands over
/// those three readings (pure reads, so where they are taken changes nothing).
fn draw_condition(state: &GameState, controller: PlayerId, threshold: i32, radiant: bool, left_now: i32) -> bool {
    let mana = if radiant { max_mana_of(state, controller) } else { left_now };
    mana >= threshold
}

fn burn(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let mut effects = vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": param(ctx, "damage") })))];
            let threshold = param(ctx, "threshold");
            let left = unspent_mana_of(&ctx.state, ctx.controller);
            if draw_condition(&ctx.state, ctx.controller, threshold, radiant, left) {
                effects.push(draw(json_as(json!({ "count": param(ctx, "draw") }))));
            }
            effects
        })),
        // R195: in hand, whether playing it now would draw.
        condition_met: Some(condition_hook(move |c: ConditionContext| -> bool {
            c.zone == ConditionZone::Hand
                && draw_condition(
                    c.state,
                    c.controller,
                    param(&c, "threshold"),
                    radiant,
                    unspent_mana_of(c.state, c.controller) - effective_cost(c.state, c.self_, Default::default()),
                )
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = burn(false);

    let radiant = burn(true);

    CardScripts { base, radiant }
}

// C #36 Burn — SPEC §8.6 row 36, BUILD M9 Classic row C 36: "A target, Unit or hero: deal 2, then draw
// 1 if your current mana as it resolves is 4 or more; `conditionMet` answers in hand whether your mana
// after paying its cost now would be 4 or more (R195), so a surcharge (C #77) moves it; radiant: deal
// 4, and draw if your max mana is 4 or more (§2.3); its name is a rules word, and "burned" elsewhere
// is no reference to it (R381); its tuned numbers (damage, threshold, draw) read through `param()`
// (R386)".
//
// The `conditionMet` proofs (R195) are in `../condition-active.test.ts`, with the other cards'.
#[cfg(test)]
mod tests {
    use super::{script, ID};
    use jackioh_engine::testkit::*;

    const BURN: &str = "classic-036";
    const MENACE: &str = "core-019";
    const ANCHOR: &str = "core-010";
    const A: &str = "core-020";
    const B: &str = "core-001";

    use crate::js;

    /// TS `AT_HERO: Selection[]`.
    fn at_hero() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    fn hand_defs(s: &Scenario) -> Vec<String> {
        s.hand(PlayerId::P1).iter().map(|card| card.def_id.clone()).collect()
    }

    mod c_36_burn {
        use super::*;

        #[test]
        fn declares_one_target_any_unit_or_hero_on_either_side_and_a_script_per_face() {
            crate::register_all();
            assert_eq!(js(&registered_catalog()[ID])["id"], BURN);
            let scripts = script();
            assert_eq!(
                js(&scripts.base.targets),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "hero"] } }]),
            );
            assert_eq!(js(&scripts.radiant.targets), js(&scripts.base.targets));
        }

        #[test]
        fn r381_its_name_is_a_rules_word_it_is_the_one_card_named_burn_and_no_entrys_refs_name_it() {
            crate::register_all();
            let named: Vec<Value> = registered_catalog()
                .values()
                .map(js)
                .filter(|card| card["name"] == "Burn")
                .map(|card| card["id"].clone())
                .collect();
            assert_eq!(named, vec![json!(BURN)]);
            let referring: Vec<Value> = registered_catalog()
                .values()
                .map(js)
                .filter(|card| card["refs"].as_array().is_some_and(|refs| refs.iter().any(|id| id == BURN)))
                .collect();
            assert!(referring.is_empty());
        }

        mod base {
            use super::*;

            #[test]
            fn deals_2_to_a_unit_then_with_4_mana_left_draws_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [BURN, ANCHOR], "library": [A, B] },
                    "p2": { "hand": [ANCHOR], "field": [MENACE] },
                }));
                let menace = s.card(MENACE).id.clone();

                s.play(BURN, json!({ "targets": [{ "pick": "instance", "instanceId": menace }] }));

                s.expect_stats(&menace, json!({ "health": 7 }));
                assert_eq!(hand_defs(&s), vec![ANCHOR, A]);
                s.expect_events(json!(["damage", "drawn"]));
            }

            #[test]
            fn deals_2_to_a_hero_either_side() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [BURN, BURN, ANCHOR], "library": [A, B] },
                    "p2": { "hand": [ANCHOR] },
                }));
                let (first, second) = {
                    let hand = s.hand(PlayerId::P1);
                    match (hand.first(), hand.get(1)) {
                        (Some(first), Some(second)) => (first.id.clone(), second.id.clone()),
                        _ => panic!("two Burns"),
                    }
                };

                s.play(&first, json!({ "targets": at_hero() }));
                s.play(&second, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));

                s.expect_health(PlayerId::P2, 28);
                s.expect_health(PlayerId::P1, 28);
            }

            #[test]
            fn with_3_mana_left_it_draws_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [BURN, ANCHOR], "library": [A], "mana": 3 },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(BURN, json!({ "targets": at_hero() }));

                s.expect_health(PlayerId::P2, 28);
                assert_eq!(hand_defs(&s), vec![ANCHOR]);
            }

            #[test]
            fn mana_left_is_after_paying_a_burn_made_to_cost_1_leaves_3_of_4_and_draws_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": BURN, "costMod": 1 }, ANCHOR], "library": [A] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(BURN, json!({ "targets": at_hero() }));

                s.expect_mana(PlayerId::P1, 3);
                assert_eq!(hand_defs(&s), vec![ANCHOR]);
            }

            #[test]
            fn s2_3_temporary_mana_above_max_counts_as_mana_left() {
                crate::register_all();
                let mut s = scenario(json!({
                    "turn": 3,
                    "p1": { "hand": [BURN, ANCHOR], "library": [A], "mana": 5 },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(BURN, json!({ "targets": at_hero() }));

                assert_eq!(hand_defs(&s), vec![ANCHOR, A]);
            }

            #[test]
            fn r386_an_upgrade_deals_3_a_degrade_of_the_threshold_to_5_stops_4_mana_drawing_an_upgrade_of_draw_draws_2() {
                crate::register_all();
                let mut dmg = scenario(json!({ "p1": { "hand": [BURN, ANCHOR], "library": [A, B] }, "p2": { "hand": [ANCHOR] } }));
                step_param(dmg.card_mut(BURN), "damage", 1);
                dmg.play(BURN, json!({ "targets": at_hero() }));
                dmg.expect_health(PlayerId::P2, 27);

                let mut harder = scenario(json!({ "p1": { "hand": [BURN, ANCHOR], "library": [A, B] }, "p2": { "hand": [ANCHOR] } }));
                step_param(harder.card_mut(BURN), "threshold", 1);
                harder.play(BURN, json!({ "targets": at_hero() }));
                assert_eq!(hand_defs(&harder), vec![ANCHOR]);

                let mut more = scenario(json!({ "p1": { "hand": [BURN, ANCHOR], "library": [A, B] }, "p2": { "hand": [ANCHOR] } }));
                step_param(more.card_mut(BURN), "draw", 1);
                more.play(BURN, json!({ "targets": at_hero() }));
                assert_eq!(hand_defs(&more), vec![ANCHOR, A, B]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn deals_4_and_with_max_mana_4_draws_1_even_with_no_mana_left() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": BURN, "radiant": true }, ANCHOR], "library": [A], "mana": 0 },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(BURN, json!({ "targets": at_hero() }));

                s.expect_health(PlayerId::P2, 26);
                assert_eq!(hand_defs(&s), vec![ANCHOR, A]);
            }

            #[test]
            fn s2_3_with_max_mana_3_it_draws_nothing_however_much_mana_is_left() {
                crate::register_all();
                let mut s = scenario(json!({
                    "turn": 5,
                    "p1": { "hand": [{ "def": BURN, "radiant": true }, ANCHOR], "library": [A], "mana": 9 },
                    "p2": { "hand": [ANCHOR] },
                }));
                assert_eq!(s.view(PlayerId::P1).you.mana.max, 3);

                s.play(BURN, json!({ "targets": at_hero() }));

                s.expect_health(PlayerId::P2, 26);
                assert_eq!(hand_defs(&s), vec![ANCHOR]);
            }

            #[test]
            fn r386_a_degrade_of_the_threshold_to_5_stops_max_mana_4_drawing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": BURN, "radiant": true }, ANCHOR], "library": [A] },
                    "p2": { "hand": [ANCHOR] },
                }));
                step_param(s.card_mut(BURN), "threshold", 1);

                s.play(BURN, json!({ "targets": at_hero() }));

                assert_eq!(hand_defs(&s), vec![ANCHOR]);
            }
        }
    }
}
