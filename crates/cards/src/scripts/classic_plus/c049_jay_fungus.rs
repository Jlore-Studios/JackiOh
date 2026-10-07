//! C+ #49 Jay Fungus (SPEC §8.7 row 49). (2) Unit, Rare, 3/6 → 6/12.
//!   Base:    "Taunt. End of turn: A random card in your hand costs ({discount}) less." — discount 2
//!   Radiant: the same text, discount 20.
//!   Engine:  "`costMod` −2 (Radiant −20) on a random card in your hand it can make cheaper: current cost
//!            above 0 and not an X-cost card (R65: modifiers never reach X); with none, nothing. The cost
//!            floors at 0 (§2.3). Tunes: discount 2 ↑."
//!
//! "End of turn" is its controller's (§6.2), so the opponent's turn end does nothing. The pick and the
//! R129 rule that an empty choice draws nothing are `discountRandomInHand`'s (effects/perks.ts).

use jackioh_engine::effects::discount_random_in_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-049";

pub fn script() -> CardScripts {
    let base = Script {
        end_of_turn: Some(hook(|ctx| {
            vec![discount_random_in_hand(json_as(json!({ "amount": param(&*ctx, "discount") })))]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face's 20 is its declared `discount`, which `param` reads.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C+ #49 Jay Fungus — SPEC §8.7 row 49, BUILD M9 Classic+ row C+ 49: "Taunt; at the end of your turn
// one random hand card whose cost is above 0 and not X gets `costMod` −2 (R65), the cost flooring at
// 0; no such card, nothing and no random draw (R129); the opponent's `costChanged` reads the sentinel
// and −1 (R177); the discount reads through `param()`; radiant −20, taking any such card to (0)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FUNGUS: &str = "classicplus-049";
    const X_SPELL: &str = "core-024"; // Efficiency Dividend, X Cost
    const FREE: &str = "core-010"; // Rapid Replenish, (0)
    const THREE: &str = "core-016"; // Hit Job, (3)
    const ONE: &str = "core-005"; // Stockpile, (1)

    /// TS `grown({ radiant?, hand?, fungus?, seed? })`'s options.
    struct Grown {
        radiant: bool,
        hand: Option<Vec<&'static str>>,
        fungus: bool,
        seed: Option<String>,
    }

    impl Default for Grown {
        /// TS `fungus` is on unless it is `false`.
        fn default() -> Grown {
            Grown {
                radiant: false,
                hand: None,
                fungus: true,
                seed: None,
            }
        }
    }

    fn grown(opts: Grown) -> Scenario {
        let mut fungus = json!({ "def": FUNGUS });
        if opts.radiant {
            fungus["radiant"] = json!(true);
        }
        let field = if opts.fungus { json!([fungus]) } else { json!([]) };
        scenario(json!({
            "seed": opts.seed.unwrap_or_else(|| "jay-fungus".to_string()),
            "p1": {
                "field": field,
                "hand": opts.hand.unwrap_or_else(|| vec![X_SPELL, FREE, THREE]),
                "library": [ONE, ONE],
            },
            "p2": { "hand": [ONE], "library": [ONE, ONE] },
        }))
    }

    fn cost_changes(s: &Scenario) -> Vec<(String, i32)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CostChanged { instance_id, cost, .. } => Some((instance_id.clone(), *cost)),
                _ => None,
            })
            .collect()
    }

    use crate::js;

    #[test]
    fn is_a_3_6_taunt_6_12_radiant_whose_two_faces_run_one_script() {
        crate::register_all();
        assert_eq!(crate::card_def(super::ID).id, FUNGUS);
        let mut s = grown(Grown::default());
        assert!(s.stats(FUNGUS).keywords.contains(&Keyword::Taunt));
        s.expect_stats(FUNGUS, json!({ "attack": 3, "health": 6 }));
        // TS `expect(radiant).toBe(base)`: both faces hold the very same hook.
        let scripts = script();
        assert!(Arc::ptr_eq(
            scripts.radiant.end_of_turn.as_ref().unwrap(),
            scripts.base.end_of_turn.as_ref().unwrap()
        ));
    }

    mod base {
        use super::*;

        #[test]
        fn r65_at_the_end_of_your_turn_the_one_hand_card_above_0_that_is_not_x_costs_2_less() {
            crate::register_all();
            let mut s = grown(Grown::default());
            s.end_turn();
            let three = s.card(THREE).clone();
            assert_eq!(three.cost_mod, -2);
            assert_eq!(effective_cost(s.state(), &three, Default::default()), 1);
            assert_eq!(s.card(X_SPELL).cost_mod, 0);
            assert_eq!(s.card(FREE).cost_mod, 0);
            assert_eq!(cost_changes(&s), vec![(three.id.clone(), 1)]);
        }

        #[test]
        fn s2_3_the_cost_floors_at_0_a_1_cost_card_goes_to_0() {
            crate::register_all();
            let mut s = grown(Grown {
                hand: Some(vec![ONE]),
                ..Grown::default()
            });
            s.end_turn();
            let one = s.hand(P1).into_iter().find(|card| card.def_id == ONE);
            assert_eq!(one.as_ref().map(|card| card.cost_mod), Some(-2));
            assert_eq!(
                one.as_ref()
                    .map(|card| effective_cost(s.state(), card, Default::default()))
                    .unwrap_or(-1),
                0
            );
        }

        #[test]
        fn s2_3_two_fungi_stack_on_one_card_costmod_adds_up_2_then_2() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [FUNGUS, FUNGUS], "hand": [THREE], "library": [ONE, ONE] },
                "p2": { "hand": [ONE], "library": [ONE, ONE] },
            }));
            s.end_turn();
            let three = s.card(THREE).clone();
            assert_eq!(three.cost_mod, -4);
            assert_eq!(effective_cost(s.state(), &three, Default::default()), 0);
            assert_eq!(cost_changes(&s), vec![(three.id.clone(), 1), (three.id.clone(), 0)]);
        }

        #[test]
        fn r60_the_card_is_random_among_those_it_can_make_cheaper() {
            crate::register_all();
            let mut picked: IndexSet<i64> = IndexSet::new();
            for i in 0..40 {
                let mut s = grown(Grown {
                    hand: Some(vec![THREE, THREE, THREE]),
                    seed: Some(format!("fungus-{i}")),
                    ..Grown::default()
                });
                s.end_turn();
                let at = s.hand(P1).iter().position(|card| card.cost_mod == -2);
                picked.insert(at.map(|at| at as i64).unwrap_or(-1));
            }
            let mut picked: Vec<i64> = picked.into_iter().collect();
            picked.sort();
            assert_eq!(picked, [0, 1, 2]);
        }

        #[test]
        fn r129_no_card_it_can_make_cheaper_nothing_changes_and_no_random_number_is_drawn() {
            crate::register_all();
            let mut with_fungus = grown(Grown {
                hand: Some(vec![X_SPELL, FREE]),
                ..Grown::default()
            });
            let mut without = grown(Grown {
                hand: Some(vec![X_SPELL, FREE]),
                fungus: false,
                ..Grown::default()
            });
            with_fungus.end_turn();
            without.end_turn();
            assert!(cost_changes(&with_fungus).is_empty());
            assert_eq!(with_fungus.state().rng_cursor, without.state().rng_cursor);
        }

        #[test]
        fn s6_2_the_opponent_s_end_of_turn_does_nothing() {
            crate::register_all();
            let mut s = grown(Grown::default());
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert!(cost_changes(&s).is_empty());
            assert_eq!(s.card(THREE).cost_mod, -2);
        }

        #[test]
        fn r177_the_opponent_s_costchanged_names_no_card_and_reads_1() {
            crate::register_all();
            let mut s = grown(Grown::default());
            s.end_turn();
            let theirs: Vec<GameEvent> = s
                .view(P2)
                .events
                .into_iter()
                .filter(|event| matches!(event, GameEvent::CostChanged { .. }))
                .collect();
            assert_eq!(js(&theirs), json!([{ "type": "costChanged", "instanceId": "hidden", "cost": -1 }]));
            let mine: Vec<GameEvent> = s
                .view(P1)
                .events
                .into_iter()
                .filter(|event| matches!(event, GameEvent::CostChanged { .. }))
                .collect();
            let three = s.card(THREE).id.clone();
            assert_eq!(js(&mine), json!([{ "type": "costChanged", "instanceId": three, "cost": 1 }]));
        }

        #[test]
        fn r386_an_upgrade_makes_it_3_less_a_degrade_1_less_never_below_1() {
            crate::register_all();
            let mut up = grown(Grown::default());
            step_param(up.card_mut(FUNGUS), "discount", 1);
            up.end_turn();
            assert_eq!(up.card(THREE).cost_mod, -3);

            let mut down = grown(Grown::default());
            step_param(down.card_mut(FUNGUS), "discount", -5);
            down.end_turn();
            assert_eq!(down.card(THREE).cost_mod, -1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn t_6_12_taunt_the_card_costs_20_less_which_takes_any_such_card_to_0() {
            crate::register_all();
            let mut s = grown(Grown {
                radiant: true,
                ..Grown::default()
            });
            s.expect_stats(FUNGUS, json!({ "attack": 6, "health": 12 }));
            s.end_turn();
            let three = s.card(THREE).clone();
            assert_eq!(three.cost_mod, -20);
            assert_eq!(effective_cost(s.state(), &three, Default::default()), 0);
            assert_eq!(s.card(X_SPELL).cost_mod, 0);
        }

        #[test]
        fn r129_the_radiant_face_skips_0_and_x_cards_too_drawing_nothing() {
            crate::register_all();
            let mut with_fungus = grown(Grown {
                radiant: true,
                hand: Some(vec![FREE, X_SPELL]),
                ..Grown::default()
            });
            let mut without = grown(Grown {
                hand: Some(vec![FREE, X_SPELL]),
                fungus: false,
                ..Grown::default()
            });
            with_fungus.end_turn();
            without.end_turn();
            assert!(with_fungus.hand(P1).iter().all(|card| card.cost_mod == 0));
            assert_eq!(with_fungus.state().rng_cursor, without.state().rng_cursor);
        }
    }
}
