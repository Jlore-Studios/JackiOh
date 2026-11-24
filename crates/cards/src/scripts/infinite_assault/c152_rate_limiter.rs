//! SPEC §8.13 IA #152 Rate Limiter ((2) Unit, Common, 3/5 → 6/10), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt. Your opponent can't attack with more than three Units each turn." (limit 3)
//!   Radiant: "Taunt. Your opponent can't attack with more than two Units each turn." (limit 2)
//!
//! Engine: a static flag capping the opponent's declared attacks this turn (R4591002): a Unit
//! already counted may attack again (Windfury), Units that attacked and then left the field still
//! count, and with several limits the lowest holds. Forced attacks are neither counted nor refused.
//! Taunt is the catalog's data.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-152";

/// How many Units the opponent may attack with each turn on this face.
const BASE_LIMIT: i32 = 3;
/// How many Units the opponent may attack with each turn on the Radiant face.
const RADIANT_LIMIT: i32 = 2;

fn limiter(limit: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            opponent_attack_limit: Some(limit),
            ..Default::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: limiter(BASE_LIMIT),
        radiant: limiter(RADIANT_LIMIT),
    }
}

// IA #152 Rate Limiter — SPEC §8.13 row 152 (R4591002).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const LIMITER: &str = "infiniteassault-152";
    const WEAK: &str = "stand-in (1) 1/1";
    const FILLER: &str = "stand-in (0) Spell";

    /// p1 holds the limiter; p2 holds four 1-attack Units (the limiter survives their attacks).
    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "rate-limiter",
            "active": "p2",
            "p1": {
                "field": [{ "def": LIMITER, "radiant": radiant }],
                "hand": [FILLER],
                "library": filler(6),
            },
            "p2": {
                "field": [WEAK, WEAK, WEAK, WEAK],
                "hand": [FILLER],
                "library": filler(6),
            },
        }))
    }

    fn attacks(s: &Scenario, player: PlayerId) -> Vec<ActionBody> {
        legal_actions(s.state(), player)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Attack { .. }))
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4591002_a_fourth_unit_cannot_attack() {
            let mut s = board(false);
            assert_eq!(attacks(&s, P2).len(), 4);
            for lane in 1..=3 {
                let attacker = s.unit_at(P2, lane);
                let target = s.unit_at(P1, 1);
                s.attack(&attacker, &target);
            }
            assert_eq!(attacks(&s, P2).len(), 0, "the fourth unit is refused");
            let fourth = s.unit_at(P2, 4);
            let target = s.unit_at(P1, 1);
            s.expect_refused(|s| s.attack(&fourth, &target));
        }

        #[test]
        fn r4591002_a_unit_that_attacked_and_died_still_counts() {
            let mut s = scenario(json!({
                "seed": "rate-limiter-dies",
                "active": "p2",
                "p1": {
                    "field": [LIMITER],
                    "hand": [FILLER],
                    "library": filler(6),
                },
                "p2": {
                    "field": [WEAK, WEAK, WEAK, WEAK],
                    "hand": [FILLER],
                    "library": filler(6),
                },
            }));
            // Three attacks into the limiter's strike-back; each 1/1 dies.
            for lane in 1..=3 {
                let attacker = s.unit_at(P2, lane);
                s.attack(&attacker, &s.unit_at(P1, 1));
            }
            assert!(s.unit(P2, 3).is_none(), "the third attacker died");
            assert_eq!(attacks(&s, P2).len(), 0, "the dead attacker still counts");
            let fourth = s.unit_at(P2, 4);
            let target = s.unit_at(P1, 1);
            s.expect_refused(|s| s.attack(&fourth, &target));
        }

        #[test]
        fn r4591002_a_windfury_unit_may_attack_twice() {
            let mut s = scenario(json!({
                "seed": "rate-limiter-windfury",
                "active": "p2",
                "p1": {
                    "field": [LIMITER],
                    "hand": [FILLER],
                    "library": filler(6),
                },
                "p2": {
                    "field": ["stand-in (1) 1/7 Windfury", WEAK, WEAK],
                    "hand": [FILLER],
                    "library": filler(6),
                },
            }));
            // The Windfury unit survives the limiter's strike-back twice (7 - 3 - 3).
            let windy = s.unit_at(P2, 1);
            let target = s.unit_at(P1, 1);
            s.attack(&windy, &target);
            s.attack(&windy, &target);
            let second = s.unit_at(P2, 2);
            s.attack(&second, &target);
            assert_eq!(attacks(&s, P2).len(), 0, "three attacks fill the count");
            let third = s.unit_at(P2, 3);
            s.expect_refused(|s| s.attack(&third, &target));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4591002_two_units() {
            let mut s = board(true);
            for lane in 1..=2 {
                let attacker = s.unit_at(P2, lane);
                let target = s.unit_at(P1, 1);
                s.attack(&attacker, &target);
            }
            assert_eq!(attacks(&s, P2).len(), 0, "the third unit is refused");
            let third = s.unit_at(P2, 3);
            let target = s.unit_at(P1, 1);
            s.expect_refused(|s| s.attack(&third, &target));
        }
    }
}
