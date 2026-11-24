//! IA #210 Matrioshka Brain (SPEC §8.13 row 210), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "GIGA, Taunt"
//!   Radiant: "GIGA, Taunt, Divine Shield"
//!
//! GIGA, Taunt and the Radiant face's Divine Shield are catalog data: no script. Built by C9
//! (#4593).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-210";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds Divine Shield and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #210 Matrioshka Brain — SPEC §8.13 row 210.
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
        fn a_10_20_with_giga_and_taunt() {
            let s = standing(false, "matrioshka-base");
            let unit = s.unit_at(P1, 1);
            let view = unit_view(s.state(), &unit);
            assert_eq!((view.attack, view.health), (10, 20));
            let keywords = card_keywords(s.state(), &unit);
            assert!(keywords.iter().any(|keyword| *keyword == Keyword::Giga));
            assert!(keywords.iter().any(|keyword| *keyword == Keyword::Taunt));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_20_40_with_giga_taunt_and_divine_shield() {
            let s = standing(true, "matrioshka-radiant");
            let unit = s.unit_at(P1, 1);
            let view = unit_view(s.state(), &unit);
            assert_eq!((view.attack, view.health), (20, 40));
            let keywords = card_keywords(s.state(), &unit);
            assert!(keywords.iter().any(|keyword| *keyword == Keyword::Giga));
            assert!(keywords.iter().any(|keyword| *keyword == Keyword::Taunt));
            assert!(
                keywords
                    .iter()
                    .any(|keyword| *keyword == Keyword::DivineShield)
            );
        }
    }
}
