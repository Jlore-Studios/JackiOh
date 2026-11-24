//! SPEC §8.13 IA #74.1 Instance ((1) Unit token, 2/2 → 4/4), a token of the held Infinite Assault set (R4577000).
//!   Base:    ""
//!   Radiant: ""
//!
//! A vanilla token. IA #74 Parallel Instance, #81 Overprovision and #88 Scale Out summon it.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-074-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #74.1 Instance — SPEC §7 (the Infinite Assault's tokens), §8.13 row 74.1.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn board(radiant: bool) -> Scenario {
        scenario(json!({ "p1": { "field": [{ "def": ID, "radiant": radiant }] } }))
    }

    mod base {
        use super::*;

        #[test]
        fn is_a_vanilla_2_2() {
            let s = board(false);
            let instance = s.unit_at(P1, 1);
            assert!(s.stats(&instance).keywords.is_empty());
            assert_eq!((s.stats(&instance).attack, s.stats(&instance).health), (2, 2));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_a_vanilla_4_4() {
            let s = board(true);
            let instance = s.unit_at(P1, 1);
            assert!(s.stats(&instance).keywords.is_empty());
            assert_eq!((s.stats(&instance).attack, s.stats(&instance).health), (4, 4));
        }
    }
}
