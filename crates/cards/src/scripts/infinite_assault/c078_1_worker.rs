//! SPEC §8.13 IA #78.1 Worker ((1) Unit token, 1/2 → 2/4), a token of the held Infinite Assault set (R4577000).
//!   Base:    ""
//!   Radiant: ""
//!
//! A vanilla token. IA #78 Batch Job and #87 Swarm Shield summon it.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-078-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #78.1 Worker — SPEC §7 (the Infinite Assault's tokens), §8.13 row 78.1.
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
        fn is_a_vanilla_1_2() {
            let s = board(false);
            let worker = s.unit_at(P1, 1);
            assert!(s.stats(&worker).keywords.is_empty());
            assert_eq!((s.stats(&worker).attack, s.stats(&worker).health), (1, 2));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_a_vanilla_2_4() {
            let s = board(true);
            let worker = s.unit_at(P1, 1);
            assert!(s.stats(&worker).keywords.is_empty());
            assert_eq!((s.stats(&worker).attack, s.stats(&worker).health), (2, 4));
        }
    }
}
