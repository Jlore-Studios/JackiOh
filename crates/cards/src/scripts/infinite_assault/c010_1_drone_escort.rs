//! SPEC §8.13 IA #10.1 Drone Escort ((2) Unit token, 3/3 → 6/6), a token of the held Infinite Assault set (R4577000).
//!   Base:    ""
//!   Radiant: ""
//!
//! A vanilla token, so the script is empty. IA #10 Assault Carrier's Death summons two. Not a Drone:
//! #15 Drone Foundry counts #4.1 alone (R4577004).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-010-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #10.1 Drone Escort — SPEC §7 (the Infinite Assault's tokens), §8.13 row 10.1.
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
        fn is_a_vanilla_3_3() {
            let s = board(false);
            let escort = s.unit_at(P1, 1);
            assert!(s.stats(&escort).keywords.is_empty());
            assert_eq!((s.stats(&escort).attack, s.stats(&escort).health), (3, 3));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_a_vanilla_6_6() {
            let s = board(true);
            let escort = s.unit_at(P1, 1);
            assert!(s.stats(&escort).keywords.is_empty());
            assert_eq!((s.stats(&escort).attack, s.stats(&escort).health), (6, 6));
        }
    }
}
