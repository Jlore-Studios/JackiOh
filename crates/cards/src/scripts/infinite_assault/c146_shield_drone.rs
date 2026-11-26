//! SPEC §8.13 IA #146 Shield Drone ((1) Unit, Common, 1/2 → 2/4), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Give a friendly Unit Divine Shield."
//!   Radiant: "Cry: Give a friendly Unit Divine Shield and Taunt."
//!
//! Engine: the friendly Unit picked with the play (R656, aimed to help), granted Divine Shield (the
//! Radiant face: and Taunt) as it resolves.

use jackioh_engine::effects::grant_keyword;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-146";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Help),
        ..TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"] }))
    }]
}

fn shield(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |_ctx| {
            let mut effects = vec![grant_keyword(json_as(
                json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Divine Shield" } }),
            ))];
            if radiant {
                effects.push(grant_keyword(json_as(
                    json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Taunt" } }),
                )));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: shield(false),
        radiant: shield(true),
    }
}

// IA #146 Shield Drone — SPEC §8.13 row 146.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";
    const FOE: &str = "stand-in (1) 3/3";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [UNIT, UNIT],
                "library": crate::filler(3),
            },
            "p2": { "field": [FOE], "library": crate::filler(3) },
        }))
    }

    fn shielded(s: &Scenario, unit: &CardInstance) -> bool {
        has_keyword(&s.stats(unit).keywords, KeywordKind::DivineShield)
    }

    fn taunting(s: &Scenario, unit: &CardInstance) -> bool {
        has_keyword(&s.stats(unit).keywords, KeywordKind::Taunt)
    }

    mod base {
        use super::*;

        #[test]
        fn the_chosen_friendly_unit_gains_divine_shield_and_no_taunt() {
            let mut s = board(false);
            let unit = s.unit_at(P1, 1);
            s.play(ID, pick::targets(pick::instance(&unit)));
            let drone = s.unit_at(P1, 3);
            s.expect_stats(&drone, json!({ "attack": 1, "health": 2 }));
            assert!(shielded(&s, &s.unit_at(P1, 1)));
            assert!(!taunting(&s, &s.unit_at(P1, 1)));
            assert!(!shielded(&s, &s.unit_at(P1, 2)));
        }

        #[test]
        fn r81_an_enemy_unit_is_no_legal_pick() {
            let s = board(false);
            let enemy = s.unit_at(P2, 1);
            assert!(!pick::instance_ids(&s.play_targets(ID)).contains(&enemy.id));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_chosen_friendly_unit_gains_divine_shield_and_taunt() {
            let mut s = board(true);
            let unit = s.unit_at(P1, 1);
            s.play(ID, pick::targets(pick::instance(&unit)));
            let drone = s.unit_at(P1, 3);
            s.expect_stats(&drone, json!({ "attack": 2, "health": 4 }));
            assert!(shielded(&s, &s.unit_at(P1, 1)));
            assert!(taunting(&s, &s.unit_at(P1, 1)));
        }
    }
}
