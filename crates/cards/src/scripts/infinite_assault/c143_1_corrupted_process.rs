//! SPEC §8.13 IA #143.1 Corrupted Process ((1) Unit token, 0/1 → 0/1), a token of the held Infinite Assault set (R4577000).
//!   Base:    "(IA #143 Zero-Click's token.)"
//!   Radiant: "(IA #143 Zero-Click's token.)"
//!
//! Keywords only (the catalog's Can't attack), so the script is empty. IA #143 Zero-Click writes
//! it, never a hand.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-143-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #143.1 Corrupted Process — SPEC §8.13 row 143.1.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P2;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    /// A field of two Corrupted Processes, one per face.
    fn board(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "active": "p2",
            "p2": { "field": [{ "def": ID, "radiant": radiant }, { "def": ID, "radiant": radiant }] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn no_attack_is_offered() {
            let s = board(false, "corrupted-process");
            for lane in [1, 2] {
                assert!(s.attacks_of(&s.unit_at(P2, lane)).is_empty());
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn no_attack_is_offered() {
            let s = board(true, "corrupted-process-radiant");
            for lane in [1, 2] {
                assert!(s.attacks_of(&s.unit_at(P2, lane)).is_empty());
            }
        }
    }
}
