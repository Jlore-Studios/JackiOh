//! IA #200.1 Ascendant (SPEC §8.13 row 200.1), the token IA #200 Superintelligence Seed
//! becomes when it awakens, a card of the held Infinite Assault set (R4577000).
//!
//! A vanilla 12/12, 24/24 Radiant: no script. Built by C9 (#4593).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-200-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #200.1 Ascendant — SPEC §8.13 row 200.1.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    fn standing(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "field": [{ "def": ID, "lane": 1, "radiant": radiant }] },
            "p2": {},
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn a_vanilla_12_12() {
            let s = standing(false, "ascendant-base");
            let unit = s.unit_at(P1, 1);
            let view = unit_view(s.state(), &unit);
            assert_eq!((view.attack, view.health), (12, 12));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_vanilla_24_24() {
            let s = standing(true, "ascendant-radiant");
            let unit = s.unit_at(P1, 1);
            let view = unit_view(s.state(), &unit);
            assert_eq!((view.attack, view.health), (24, 24));
        }
    }
}
