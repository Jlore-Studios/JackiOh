//! SPEC §8.13 IA #147 Hardened Node ((2) Unit, Common, 2/7 → 4/14), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt, Armor 2"
//!   Radiant: "Taunt, Armor 4"
//!
//! Keywords only, so the script is empty: they are the catalog's data (§6.1).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-147";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats and the Armor: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #147 Hardened Node — SPEC §8.13 row 147.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "stand-in (0) Spell";
    const HITTER: &str = "stand-in (2) 4/4";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": crate::filler(2) },
            "p2": { "hand": [FILLER], "field": [HITTER], "library": crate::filler(2) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn a_2_7_taunt_that_armors_2() {
            let mut s = board(false);
            s.play(ID, json!({ "zone": 1 }));
            let node = s.unit_at(P1, 1);
            s.expect_stats(&node, json!({ "attack": 2, "health": 7 }));
            s.end_turn();
            let hitter = s.unit_at(P2, 1);
            s.attack(&hitter, &node);
            // 4 attack past 2 Armor: 2 lands.
            s.expect_stats(&node, json!({ "attack": 2, "health": 5 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_4_14_taunt_that_armors_4() {
            let mut s = board(true);
            s.play(ID, json!({ "zone": 1 }));
            let node = s.unit_at(P1, 1);
            s.expect_stats(&node, json!({ "attack": 4, "health": 14 }));
            s.end_turn();
            let hitter = s.unit_at(P2, 1);
            s.attack(&hitter, &node);
            // 4 attack past 4 Armor: nothing lands.
            s.expect_stats(&node, json!({ "attack": 4, "health": 14 }));
        }
    }
}
