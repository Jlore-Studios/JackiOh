//! S #10.3 Power Word Shield (SPEC §8.10 row S 10.3; §2.4): (0) Spell, Other-Origins, Token
//! (printed Epic).
//!   Base and Radiant: "Give a Unit +{health} Health. Draw {draw|card|cards}." — +2 and 1 card, +4
//!   and 2 on the Radiant face.
//!
//! Engine: a declared target, any Unit on either side (R81); `buff` its Health by the declared
//! number, then `draw` the declared count (§2.4). Both faces run this one script: the Radiant
//! face's numbers are its declared `health` and `draw` (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-010-3";

/// R81: any Unit on either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            vec![
                buff(json_as(json!({
                    "target": { "of": "chosen" },
                    "health": param(&*ctx, "health"),
                }))),
                draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
            ]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's +4 and 2 cards are its declared numbers (R386).
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #10.3 Power Word Shield — SPEC §8.10 row S 10.3, BUILD M12 row S 10.3.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{def_ids, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SHIELD: &str = "stinkyscape-010-3";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005";

    mod base {
        use super::*;

        #[test]
        fn gives_2_health_current_and_max_and_draws_1() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [SHIELD, FILLER], "field": [VANILLA], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            let unit = s.unit_at(P1, 1);

            s.play(SHIELD, json!({ "targets": pick::instance(&unit) }));

            s.expect_stats(&unit, json!({ "attack": 4, "health": 6, "maxHealth": 6 }));
            assert_eq!(def_ids(&s.hand(P1)), [FILLER, FILLER]);
        }

        #[test]
        fn an_enemy_unit_may_be_chosen() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [SHIELD, FILLER], "library": [FILLER] },
                "p2": { "hand": [FILLER], "field": [VANILLA] },
            }));
            let unit = s.unit_at(P2, 1);

            s.play(SHIELD, json!({ "targets": pick::instance(&unit) }));

            s.expect_stats(&unit, json!({ "attack": 4, "health": 6, "maxHealth": 6 }));
            assert_eq!(def_ids(&s.hand(P1)), [FILLER, FILLER]);
        }

        #[test]
        fn r386_an_upgrade_of_health_and_draw_moves_both() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [SHIELD, FILLER], "field": [VANILLA], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            step_param(s.card_mut(SHIELD), "health", 1);
            step_param(s.card_mut(SHIELD), "draw", 1);
            let unit = s.unit_at(P1, 1);

            s.play(SHIELD, json!({ "targets": pick::instance(&unit) }));

            s.expect_stats(&unit, json!({ "health": 7, "maxHealth": 7 }));
            assert_eq!(s.hand(P1).len(), 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn gives_4_health_and_draws_2() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": SHIELD, "radiant": true }, FILLER],
                    "field": [VANILLA],
                    "library": [FILLER, FILLER, FILLER],
                },
                "p2": { "hand": [FILLER] },
            }));
            let unit = s.unit_at(P1, 1);

            s.play(SHIELD, json!({ "targets": pick::instance(&unit) }));

            s.expect_stats(&unit, json!({ "attack": 4, "health": 8, "maxHealth": 8 }));
            assert_eq!(def_ids(&s.hand(P1)), [FILLER, FILLER, FILLER]);
        }
    }
}
