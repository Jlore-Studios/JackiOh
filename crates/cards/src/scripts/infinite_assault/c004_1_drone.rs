//! SPEC §8.13 IA #4.1 Drone ((1) Unit token, 1/1 → 2/2), a token of the held Infinite Assault set (R4577000).
//!   Base:    ""
//!   Radiant: ""
//!
//! A vanilla token, so the script is empty. IA #4 Wave Spawner, #7 Bulk Order and #15 Drone Foundry
//! summon it; it is the only "Drone" #15 counts (R4577004).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-004-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #4.1 Drone — SPEC §7 (the Infinite Assault's tokens), §8.13 row 4.1.
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
        fn is_a_vanilla_1_1() {
            let s = board(false);
            let drone = s.unit_at(P1, 1);
            assert!(s.stats(&drone).keywords.is_empty());
            assert_eq!((s.stats(&drone).attack, s.stats(&drone).health), (1, 1));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_a_vanilla_2_2() {
            let s = board(true);
            let drone = s.unit_at(P1, 1);
            assert!(s.stats(&drone).keywords.is_empty());
            assert_eq!((s.stats(&drone).attack, s.stats(&drone).health), (2, 2));
        }
    }
}
