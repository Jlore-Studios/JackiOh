//! M #49.3 AI Girlfriend (SPEC §8.8 row 49.3): (4) Unit, CN, Token (printed
//! Mythic), 2/30 → 4/60, Can't attack.
//!
//! "Can't attack. Start of turn: Every enemy Unit attacks this. After this is attacked:
//! Nerf the attacker [Radiant: {times|time|times}]."
//! Engine: Start of turn is Moths to the Flame's `forced_attacks_on` self
//! (R53): every enemy Unit attacks in lane order, each its own combat with its own state check,
//! Can't attack and 0-attack Units included, the run stopping once this leaves the field.
//! ME-AFTER-ATTACKED (MD-C26, R1026): after each combat it was the target of, declared or forced,
//! the attacker still on the field is Nerfed (a Cleave splash is no attack on it).

use jackioh_engine::effects::{degrade, forced_attacks_on};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-049-3";

fn girlfriend() -> Script {
    // Both faces share the one script: the Radiant face's two Nerfs are its declared `times`.
    Script {
        start_of_turn: Some(hook(|_ctx| {
            vec![forced_attacks_on(json_as(json!({
                "target": { "of": "self" },
                "attackers": "enemy",
            })))]
        })),
        after_attacked: Some(hook(|ctx| {
            let Some(facts) = after_attacked_of(&*ctx) else {
                return vec![];
            };
            if !facts.attacker_survived {
                return vec![];
            }
            vec![degrade(json_as(json!({
                "instanceId": facts.attacker_id,
                "times": param(&*ctx, "times"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = girlfriend();
    CardScripts {
        base: base.clone(),
        radiant: base,
    }
}

/// R767000: through v0.5.7 the start-of-turn hook ran with the Animated backrow guard;
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_7() -> CardScripts {
    let guarded: Hook = hook(|ctx| {
        let animated = matches!(
            ctx.live_self().map(|me| &me.zone),
            Some(Zone::Field { row: Row::Units, .. })
        );
        if !animated {
            return vec![];
        }
        vec![forced_attacks_on(json_as(json!({
            "target": { "of": "self" },
            "attackers": "enemy",
        })))]
    });
    let mut base = girlfriend();
    base.start_of_turn = Some(guarded);
    CardScripts {
        base: base.clone(),
        radiant: base,
    }
}

// M #49.3 AI Girlfriend — SPEC §8.8 row 49.3, BUILD M10 row M 49.3.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GIRLFRIEND: &str = "meditative-049-3";
    const TOUGH: &str = "core-012"; // 3/4, surviving the strike-back.
    const BIG: &str = "core-043"; // 3/10, surviving even the Radiant strike-back.
    const WEAK: &str = "core-007"; // 1/1, dying to it.
    const FILLER: &str = "core-005";
    const VIRUS: &str = "meditative-049-2"; // 0/8, Can't attack.
    const CLEAVER: &str = "classic-073"; // Nurse Cleaver 3/6, Rush, Cleave, Lifesteal.

    /// The forced attacks the game declared, attacker then target, in order.
    fn forced(s: &Scenario) -> Vec<(String, String)> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AttackDeclared {
                    attacker_id,
                    target_id,
                    forced: true,
                    ..
                } => Some((attacker_id.clone(), target_id.clone())),
                _ => None,
            })
            .collect()
    }

    /// p1 plays the Girlfriend into unit zone `lane` with `mana` to pay for it.
    fn played(seed: &str, radiant_face: bool, lane: i32) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": GIRLFRIEND, "radiant": radiant_face }, FILLER],
                "library": [FILLER, FILLER, FILLER, FILLER],
                "mana": 4,
            },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }));
        s.play(GIRLFRIEND, json!({ "zone": lane }));
        s
    }

    fn nerfed(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Degraded { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod m49_3_ai_girlfriend {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r1025_it_enters_as_a_unit_with_no_animated_keyword() {
                let s = played("girlfriend-unit", false, 2);
                // Straight into the unit zone: a Unit with no Animated keyword.
                let girlfriend = s.unit(P1, 2).expect("girlfriend");
                assert_eq!(girlfriend.def_id, GIRLFRIEND);
                let keywords =
                    serde_json::to_string(&s.stats(&girlfriend).keywords).expect("JSON");
                assert!(!keywords.contains("Animated"), "{keywords}");
                assert!(s.backrow(P1, 2).is_none());
            }

            #[test]
            fn it_cannot_attack() {
                let mut s = played("girlfriend-cannot-attack", false, 1);
                let girlfriend = s.unit(P1, 1).expect("girlfriend");
                let keywords = serde_json::to_string(&s.stats(&girlfriend).keywords).expect("JSON");
                assert!(keywords.contains("Can't attack"), "{keywords}");
                // A turn later, no longer newly summoned, it still can't.
                s.end_turn();
                s.end_turn();
                s.expect_refused(|s| s.attack(&girlfriend, "hero"));
            }

            #[test]
            fn r53_every_enemy_unit_attacks_it_in_lane_order_cant_attack_and_0_attack_included() {
                let mut s = scenario(json!({
                    "seed": "girlfriend-order",
                    "p1": {
                        "hand": [GIRLFRIEND, FILLER],
                        "library": [FILLER, FILLER],
                        "mana": 4,
                    },
                    "p2": {
                        "field": [
                            { "def": TOUGH, "lane": 1 },
                            { "def": VIRUS, "lane": 2 },
                            { "def": BIG, "lane": 4 },
                        ],
                        "hand": [FILLER],
                        "library": [FILLER, FILLER],
                    },
                }));
                s.play(GIRLFRIEND, json!({ "zone": 1 }));
                let girlfriend = s.unit(P1, 1).expect("girlfriend").id;
                let order: Vec<String> = [1, 2, 4]
                    .iter()
                    .map(|lane| s.unit(P2, *lane).expect("attacker").id)
                    .collect();

                s.start_turn();

                let expected: Vec<(String, String)> =
                    order.into_iter().map(|id| (id, girlfriend.clone())).collect();
                assert_eq!(forced(&s), expected);
            }

            #[test]
            fn r53_the_run_stops_once_it_leaves_the_field() {
                let mut s = scenario(json!({
                    "seed": "girlfriend-stops",
                    "p1": {
                        "hand": [GIRLFRIEND, FILLER],
                        "library": [FILLER, FILLER],
                        "mana": 4,
                    },
                    "p2": {
                        "field": [{ "def": TOUGH, "lane": 1 }, { "def": TOUGH, "lane": 2 }],
                        "hand": [FILLER],
                        "library": [FILLER, FILLER],
                    },
                }));
                s.play(GIRLFRIEND, json!({ "zone": 1 }));
                let girlfriend = s.unit(P1, 1).expect("girlfriend").id;
                s.card_mut(girlfriend.as_str()).damage = 28;

                s.start_turn();

                // The first attacker's 3 destroys it: the second never attacks.
                assert_eq!(forced(&s).len(), 1);
                assert!(s.unit(P1, 1).is_none());
            }

            #[test]
            fn r1026_a_declared_attack_on_it_nerfs_the_attacker_too() {
                let mut s = scenario(json!({
                    "seed": "girlfriend-declared",
                    "p1": {
                        "hand": [GIRLFRIEND, FILLER],
                        "library": [FILLER, FILLER],
                        "mana": 4,
                    },
                    "p2": {
                        "field": [{ "def": TOUGH, "lane": 1 }],
                        "hand": [FILLER],
                        "library": [FILLER, FILLER],
                    },
                }));
                s.play(GIRLFRIEND, json!({ "zone": 1 }));
                let girlfriend = s.unit(P1, 1).expect("girlfriend");
                s.end_turn();
                let tough = s.unit(P2, 1).expect("tough");

                s.attack(&tough, &girlfriend);

                assert_eq!(nerfed(&s), vec![tough.id]);
            }

            #[test]
            fn r1026_a_cleave_splash_is_no_attack_on_it() {
                let mut s = scenario(json!({
                    "seed": "girlfriend-cleave",
                    "p1": {
                        "field": [{ "def": BIG, "lane": 2 }],
                        "hand": [GIRLFRIEND, FILLER],
                        "library": [FILLER, FILLER],
                        "mana": 4,
                    },
                    "p2": {
                        "field": [{ "def": CLEAVER, "lane": 2 }],
                        "hand": [FILLER],
                        "library": [FILLER, FILLER],
                    },
                }));
                s.play(GIRLFRIEND, json!({ "zone": 1 }));
                let girlfriend = s.unit(P1, 1).expect("girlfriend");
                s.end_turn();
                let cleaver = s.unit(P2, 2).expect("cleaver");
                let big = s.unit(P1, 2).expect("big");

                s.attack(&cleaver, &big);

                // The splash hit the Girlfriend beside the target, but it was not attacked.
                assert!(s.card(&girlfriend).damage > 0, "the Cleave splash reached it");
                assert!(nerfed(&s).is_empty());
            }

            #[test]
            fn r1026_each_attacker_still_on_the_field_is_nerfed() {
                let mut s = scenario(json!({
                    "seed": "girlfriend-nerf",
                    "p1": {
                        "hand": [GIRLFRIEND, FILLER],
                        "library": [FILLER, FILLER],
                        "mana": 4,
                    },
                    "p2": {
                        "field": [
                            { "def": TOUGH, "lane": 1 },
                            { "def": WEAK, "lane": 2 },
                        ],
                        "hand": [FILLER],
                        "library": [FILLER, FILLER],
                    },
                }));
                s.play(GIRLFRIEND, json!({ "zone": 1 }));
                let tough = s.unit(P2, 1).expect("tough").clone();
                let weak = s.unit(P2, 2).expect("weak").clone();

                s.start_turn();

                assert_eq!(nerfed(&s), vec![tough.id]);
                assert!(s.unit(P2, 2).is_none());
                let _ = weak;
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r386_times_reads_through_param() {
                let mut s = scenario(json!({
                    "seed": "girlfriend-radiant-param",
                    "p1": {
                        "hand": [{ "def": GIRLFRIEND, "radiant": true }, FILLER],
                        "library": [FILLER, FILLER],
                        "mana": 4,
                    },
                    "p2": {
                        "field": [{ "def": BIG, "lane": 1 }],
                        "hand": [FILLER],
                        "library": [FILLER, FILLER],
                    },
                }));
                s.play(GIRLFRIEND, json!({ "zone": 1 }));
                let girlfriend = s.unit(P1, 1).expect("girlfriend");
                // One step down: the Radiant face's 2 becomes 1.
                step_param(s.card_mut(&girlfriend), "times", -1);

                s.start_turn();

                assert_eq!(nerfed(&s).len(), 1);
            }

            #[test]
            fn r1026_radiant_nerfs_the_attacker_twice() {
                let mut s = scenario(json!({
                    "seed": "girlfriend-radiant-nerf",
                    "p1": {
                        "hand": [{ "def": GIRLFRIEND, "radiant": true }, FILLER],
                        "library": [FILLER, FILLER],
                        "mana": 4,
                    },
                    "p2": {
                        "field": [{ "def": BIG, "lane": 1 }],
                        "hand": [FILLER],
                        "library": [FILLER, FILLER],
                    },
                }));
                s.play(GIRLFRIEND, json!({ "zone": 1 }));
                let big = s.unit(P2, 1).expect("big").clone();

                s.start_turn();

                assert_eq!(nerfed(&s), vec![big.id.clone(), big.id]);
            }
        }

        /// R767000: the script kept for the replays of v0.5.7 and before, when it was an Animated
        /// Field Spell whose forced attacks found no target from the backrow.
        mod v0_5_7 {
            use super::*;

            #[test]
            fn r767000_a_game_on_v0_5_7_folds_under_the_kept_script_a_girlfriend_left_in_the_backrow_draws_no_attack()
            {
                crate::register_all();
                jackioh_engine::with_shipped(crate::catalog_at("v0.5.7"), || {
                    let mut s = scenario(json!({
                        "seed": "girlfriend-kept",
                        "p1": {
                            "field": [
                                { "def": TOUGH, "lane": 1 },
                                { "def": TOUGH, "lane": 2 },
                                { "def": TOUGH, "lane": 3 },
                                { "def": TOUGH, "lane": 4 },
                                { "def": TOUGH, "lane": 5 },
                            ],
                            "hand": [GIRLFRIEND, FILLER],
                            "library": [FILLER, FILLER],
                            "mana": 4,
                        },
                        "p2": {
                            "field": [{ "def": TOUGH, "lane": 1 }],
                            "hand": [FILLER],
                            "library": [FILLER, FILLER],
                        },
                    }));
                    s.play(GIRLFRIEND, json!({ "zone": 1 }));
                    // No open unit zone: it stays a backrow card, and the kept guard finds its
                    // forced attacks no target.
                    let back = s.backrow(P1, 1).expect("in the backrow");
                    assert_eq!(back.def_id, GIRLFRIEND);
                    s.start_turn();
                    assert!(forced(&s).is_empty());
                    assert!(nerfed(&s).is_empty());
                });
            }
        }
    }
}
