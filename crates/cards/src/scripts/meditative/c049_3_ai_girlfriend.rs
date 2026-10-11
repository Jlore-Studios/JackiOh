//! M #49.3 AI Girlfriend (SPEC §8.8 row 49.3): (4) Field Spell, CN, Acclaimed, Token (printed
//! Mythic), 2/30 → 4/60, Animated, Can't attack.
//!
//! "Animated, Can't attack. Start of turn: Every enemy Unit attacks this. After this is attacked:
//! Nerf the attacker [Radiant: {times|time|times}]."
//! Engine: Animated (§6.1, R383) — summoned into the backrow, it animates as it enters the field
//! into its lane's unit zone (else R64's), a Unit for every rule from then on; with no open unit
//! zone it stays a backrow card. Start of turn is Moths to the Flame's `forced_attacks_on` self
//! (R53): every enemy Unit attacks in lane order, each its own combat with its own state check,
//! Can't attack and 0-attack Units included, the run stopping once this leaves the field — but only
//! while this stands animated in a unit zone, since a backrow card is no target. ME-AFTER-ATTACKED
//! (MD-C26): after each combat it was the target of, declared or forced, the attacker still on the
//! field is Nerfed (a Cleave splash is no attack on it).

use jackioh_engine::effects::{degrade, forced_attacks_on};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-049-3";

/// Whether this stands animated in a unit zone: a backrow card's forced attacks find no target.
fn animated(ctx: &EffectContext) -> bool {
    matches!(
        ctx.live_self().map(|me| &me.zone),
        Some(Zone::Field {
            row: Row::Units,
            ..
        })
    )
}

fn girlfriend() -> Script {
    // Both faces share the one script: the Radiant face's two Nerfs are its declared `times`.
    Script {
        start_of_turn: Some(hook(|ctx| {
            if !animated(ctx) {
                return vec![];
            }
            vec![forced_attacks_on(json_as(json!({
                "target": { "of": "self" },
                "attackers": "enemy",
            })))]
        })),
        after_attacked: Some(hook(|ctx| {
            let Some(facts) = after_attacked_of(&*ctx) else {
                return vec![];
            };
            // MD-C26: Nerf the attacker only if it is still on the field.
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

    /// p1 plays the Girlfriend into backrow `lane` with `mana` to pay for it.
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

    /// The ids the game Nerfed, in order.
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
            fn r1025_it_animates_into_a_unit_zone_or_stays_in_the_backrow() {
                let s = played("girlfriend-animates", false, 2);
                // Into its lane's unit zone: a Unit for every rule from then on.
                let unit = s.unit(P1, 2).expect("animated");
                assert_eq!(unit.def_id, GIRLFRIEND);
                assert!(s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::Animated { .. }
                )));
            }

            #[test]
            fn r1025_with_no_open_unit_zone_it_stays_in_the_backrow() {
                let mut s = scenario(json!({
                    "seed": "girlfriend-no-room",
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

                // No room: a backrow card, and its forced attacks find no target.
                let back = s.backrow(P1, 1).expect("in the backrow");
                assert_eq!(back.def_id, GIRLFRIEND);
                s.start_turn();
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

                // Both attacked; the survivor is Nerfed once, the one that died to the
                // strike-back is not.
                assert_eq!(nerfed(&s), vec![tough.id]);
                assert!(s.unit(P2, 2).is_none());
                let _ = weak;
            }
        }

        mod radiant {
            use super::*;

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
    }
}
