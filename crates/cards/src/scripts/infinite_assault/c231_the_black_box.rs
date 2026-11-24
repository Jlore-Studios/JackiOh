//! IA #231 The Black Box (SPEC §8.13 row 231), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Taunt, Can't attack. Start of turn: Exile a random enemy permanent. This gains +5
//!            Health."
//!   Radiant: "Taunt, Can't attack. Start of turn: Exile two random enemy permanents. This gains +5
//!            Health."
//!
//! Taunt and Can't attack are catalog data. A permanent is any card in a unit or backrow zone — a
//! face-down card in, a Dormant one out — exiled at random (`exileRandomFrom` off the field); then
//! this gains +5 Health, with nothing to exile too (R4594010).
//!
//! Engine: `exileRandomFrom`, `buff` (R4594010). Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-231";

/// How many enemy permanents the Start of turn exiles.
const EXILES: i32 = 1;
const RADIANT_EXILES: i32 = 2;

/// The Health this gains, exiles or none.
const GAIN: i32 = 5;

fn black_box(exiles: i32) -> Script {
    Script {
        start_of_turn: Some(hook(move |_| {
            vec![
                exile_random_from(json_as(json!({
                    "player": "enemy",
                    "from": "field",
                    "count": exiles,
                }))),
                buff(json_as(json!({
                    "target": { "of": "self" },
                    "health": GAIN,
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: black_box(EXILES), radiant: black_box(RADIANT_EXILES) }
}

// IA #231 The Black Box — SPEC §8.13 row 231 (R4594010).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const VICTIM: &str = "core-008"; // Mr. Vanilla: the exiled permanent.
    const SET: &str = "stand-in (1) Trap"; // A textless face-down Trap.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn health_of(s: &Scenario) -> i32 {
        unit_view(s.state(), &s.unit_at(P1, 1)).health
    }

    mod base {
        use super::*;

        #[test]
        fn r4594010_a_face_down_trap_can_be_exiled() {
            let mut s = scenario(json!({
                "seed": "blackbox-facedown",
                "active": "p1",
                "p1": { "field": [{ "def": ID }], "hand": [STOCKPILE], "library": crate::filler(4) },
                "p2": { "backrow": [SET], "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            s.start_turn();
            s.expect_in_zone(SET, "exile");
            assert_eq!(health_of(&s), 30 + 5, "+5 Health too");
        }

        #[test]
        fn r4594010_plus_5_with_nothing_to_exile() {
            let mut s = scenario(json!({
                "seed": "blackbox-empty",
                "active": "p1",
                "p1": { "field": [{ "def": ID }], "hand": [STOCKPILE], "library": crate::filler(4) },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            s.start_turn();
            assert_eq!(health_of(&s), 30 + 5);
        }

        #[test]
        fn it_cannot_attack() {
            let s = scenario(json!({
                "seed": "blackbox-still",
                "p1": { "field": [{ "def": ID }], "hand": [STOCKPILE], "library": crate::filler(4) },
                "p2": { "field": [VICTIM], "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            assert!(s.attacks_of(ID).is_empty(), "Can't attack");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594010_two_permanents_are_exiled() {
            let mut s = scenario(json!({
                "seed": "blackbox-radiant",
                "active": "p1",
                "p1": { "field": [{ "def": ID, "radiant": true }], "hand": [STOCKPILE], "library": crate::filler(4) },
                "p2": { "field": [VICTIM], "backrow": [SET], "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            s.start_turn();
            s.expect_in_zone(VICTIM, "exile");
            s.expect_in_zone(SET, "exile");
            assert_eq!(health_of(&s), 60 + 5, "+5 Health too");
        }
    }
}
