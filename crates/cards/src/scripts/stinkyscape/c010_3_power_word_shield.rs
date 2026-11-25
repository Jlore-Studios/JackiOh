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
/// R656: Health on the target helps (the draw is on the caster), so a random cast that targets
/// enemies aims this at friends.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl { aim: Some(TargetAim::Help), ..TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] })) }]
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

/// R767000: through v0.6.13e the target declared no aim (issue #4844); `KEPT_SCRIPTS` folds
/// those replays under it.
pub fn script_v0_6_13e() -> CardScripts {
    let old = vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))];
    let now = script();
    CardScripts {
        base: Script { targets: old.clone(), ..now.base },
        radiant: Script { targets: old, ..now.radiant },
    }
}

// S #10.3 Power Word Shield — SPEC §8.10 row S 10.3, BUILD M12 row S 10.3.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{def_ids, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SHIELD: &str = "stinkyscape-010-3";
    const PALANTIR: &str = "classic-004"; // Palantir: steals a (2) Cost Spell and casts it at enemies (R831001).
    const MINE: &str = "stand-in (1) 3/3";
    const THEIRS: &str = "stand-in (1) 4/4";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005";

    mod base {
        use super::*;

        #[test]
        fn gives_2_health_current_and_max_and_draws_1() {
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

    /// A (0) Cost Spell made a (2) Cost one by a `costMod` of 2: Palantir steals and casts it (R831001).
    /// P1 holds the Spell and a Unit, P2 a Unit and a face-up Palantir: P1 plays it on their own
    /// Unit, Palantir steals it and casts it at enemies.
    fn stolen_rig() -> (Scenario, CardInstance, CardInstance) {
        let mut s = scenario(json!({
            "seed": "audit-aim",
            "p1": { "hand": [{ "def": SHIELD, "costMod": 2 }], "field": [{ "def": MINE, "lane": 1 }],
                    "library": [FILLER, FILLER, FILLER], "mana": 10 },
            "p2": { "hand": [FILLER], "field": [{ "def": THEIRS, "lane": 1 }],
                    "backrow": [{ "def": PALANTIR, "faceUp": true }], "library": [FILLER, FILLER, FILLER] },
        }));
        let mine = s.unit_at(P1, 1);
        let theirs = s.unit_at(P2, 1);
        let cast = s.hand(P1)[0].id.clone();
        s.play(cast.as_str(), pick::targets(pick::instance(&mine)));
        (s, mine, theirs)
    }

    #[test]
    fn r656_the_target_aims_help() {
        let scripts = super::script();
        for face in [&scripts.base, &scripts.radiant] {
            assert_eq!(
                serde_json::to_value(&face.targets).unwrap(),
                json!([{
                    "kind": "target",
                    "min": 1,
                    "max": 1,
                    "aim": "help",
                    "filter": { "side": "any", "of": ["unit"] },
                }]),
            );
        }
    }

    mod v0_6_13e {
        use super::*;

        use crate::scripts::stinkyscape::c010_3_power_word_shield::{ID, script_v0_6_13e};

        /// R767000: a game on v0.6.13e, before the aim, shields the caster's enemy with a stolen cast.
        #[test]
        fn r767000_a_game_on_v0_6_13e_shields_the_casters_enemy_with_a_stolen_cast() {
            crate::under_kept(ID, script_v0_6_13e(), || {
                let (mut s, mine, theirs) = stolen_rig();
                assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == SHIELD), "P2 stole and cast it");
                s.expect_stats(&mine, json!({ "attack": 3, "health": 5, "maxHealth": 5 }));
                s.expect_stats(&theirs, json!({ "attack": 4, "health": 4, "maxHealth": 4 }));
            });
        }
    }
}
