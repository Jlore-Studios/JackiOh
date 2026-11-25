//! SPEC §8.13 IA #82.1 Fork ((1) Unit token, 1/1 → 2/2), a token of the held Infinite Assault set (R4577000).
//!   Base:    ""
//!   Radiant: ""
//!
//! A vanilla token. IA #82 Fork Bomb's Death summons it.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-082-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #82.1 Fork — SPEC §7 (the Infinite Assault's tokens), §8.13 row 82.1.
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
            let fork = s.unit_at(P1, 1);
            assert!(s.stats(&fork).keywords.is_empty());
            assert_eq!((s.stats(&fork).attack, s.stats(&fork).health), (1, 1));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_a_vanilla_2_2() {
            let s = board(true);
            let fork = s.unit_at(P1, 1);
            assert!(s.stats(&fork).keywords.is_empty());
            assert_eq!((s.stats(&fork).attack, s.stats(&fork).health), (2, 2));
        }
    }
}
