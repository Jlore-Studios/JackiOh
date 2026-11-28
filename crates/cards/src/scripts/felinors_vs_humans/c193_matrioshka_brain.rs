//! SPEC §8.14 FvH #193 Plagiarized: Matrioshka Brain, a frozen copy of IA #210 Matrioshka Brain (R2195003,
//! FV4), a card of the held Felinors vs Humans set, built by #5235.
//!
//! The code is a copy of the IA card's as it stood when the set was founded, not a re-export, so a later IA
//! patch does not move it (R2195003). Its `ID` is its own, so every read of it names this card; it is named
//! `Plagiarized: Matrioshka Brain`, tagged Plagiarized only, and a token it makes stays the IA token. Its
//! change from IA #210: none. The IA file's notes, as they stood:
//!
//! IA #210 Matrioshka Brain (SPEC §8.13 row 210), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "GIGA, Taunt"
//!   Radiant: "GIGA, Taunt, Divine Shield"
//!
//! GIGA, Taunt and the Radiant face's Divine Shield are catalog data: no script. Built by C9
//! (#4593).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-193";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds Divine Shield and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #193 Plagiarized: Matrioshka Brain — SPEC §8.14 row 193; IA #210's tests (SPEC §8.13 row 210), on this
// card.
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
            assert!(keywords.contains(&Keyword::Giga));
            assert!(keywords.contains(&Keyword::Taunt));
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
            assert!(keywords.contains(&Keyword::Giga));
            assert!(keywords.contains(&Keyword::Taunt));
            assert!(
                keywords.contains(&Keyword::DivineShield)
            );
        }
    }
}
