//! IA #33 Uncanny Valley (SPEC §8.13 row 33), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Cry: Enemy Units have −{debuff} Attack until your next turn." (2)
//!   Radiant: "Cry: Enemy Units have −{debuff} Attack until your next turn." (4)
//!
//! A `unitsAttack` modifier on the opponent, read beside the auras: every enemy Unit on the
//! field has the cut, those arriving later too, gated as the War Doctrines' lend (R1859007),
//! and Attack is floored at 0 (§10.4). R757's expiry read for the caster: made on their turn it
//! lasts through the affected side's next turn; made elsewhere, this turn. Several stack.
//!
//! Engine: `giveUnitsAttack` (R4586006). Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-033";

fn valley() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let debuff = param(&*ctx, "debuff");
            vec![give_units_attack(json_as(json!({
                "player": "enemy",
                "amount": -debuff,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: valley(),
        // The same script: the Radiant face's 4 is its declared `debuff`.
        radiant: valley(),
    }
}

// IA #33 Uncanny Valley — SPEC §8.13 row 33 (R4586006).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P2;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const BIG: &str = "core-008"; // Mr. Vanilla: a 4/4.
    const SMALL: &str = "core-001"; // Big D-fender: a 0/7.

    fn played(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": { "field": [BIG], "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn attack_of(s: &Scenario, lane: i32) -> i32 {
        unit_view(s.state(), &s.unit_at(P2, lane)).attack
    }

    mod base {
        use super::*;

        #[test]
        fn r4586006_enemy_units_have_2_less_attack_and_a_late_one_too() {
            let mut s = scenario(json!({
                "seed": "valley-late",
                "p1": {
                    "hand": [ID, STOCKPILE],
                    "library": crate::filler(4),
                },
                "p2": { "field": [BIG], "hand": [SMALL], "library": crate::filler(4) },
            }));
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(attack_of(&s, 1), 2, "4 minus 2");
            // p2 plays another Unit into the cut on its own turn.
            s.end_turn();
            s.play(SMALL, json!({ "zone": 2 }));
            assert_eq!(attack_of(&s, 2), 0, "a late 0 minus 2, floored at 0");
        }

        #[test]
        fn r4586006_the_cut_is_gone_at_your_next_turn() {
            let mut s = played(false, "valley-expiry");
            assert_eq!(attack_of(&s, 1), 2, "4 minus 2");
            s.end_turn();
            s.end_turn();
            assert_eq!(attack_of(&s, 1), 4, "the −2 expired");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586006_the_radiant_cut_is_4() {
            let s = played(true, "valley-radiant");
            assert_eq!(attack_of(&s, 1), 0, "2 minus 4, floored at 0");
            let mut t = scenario(json!({
                "seed": "valley-radiant-big",
                "p1": {
                    "hand": [{ "def": ID, "radiant": true }, STOCKPILE],
                    "library": crate::filler(4),
                },
                "p2": { "field": ["stand-in (4) 7/7"], "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            t.play(ID, json!({ "zone": 1 }));
            assert_eq!(attack_of(&t, 1), 3, "7 minus 4");
        }
    }
}
