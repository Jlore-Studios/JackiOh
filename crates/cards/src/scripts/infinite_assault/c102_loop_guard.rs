//! SPEC §8.13 IA #102 Loop Guard ((3) Unit, Common, 4/6 → 8/12), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt, Reborn"
//!   Radiant: "Taunt, Reborn, Divine Shield"
//!
//! Taunt, Reborn and Divine Shield are the catalog's: the script is empty, as C+ #6 Wrong-House
//! Attacker's is, and the Radiant face only adds Divine Shield.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-102";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds Divine Shield and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #102 Loop Guard — SPEC §8.13 row 102.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn guard(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }] },
            "p2": { "library": crate::filler(2) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn taunt_and_reborn() {
            let s = guard(false);
            let keywords = &s.stats(s.unit_at(P1, 1)).keywords;
            assert!(has_keyword(keywords, KeywordKind::Taunt));
            assert!(has_keyword(keywords, KeywordKind::Reborn));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn taunt_reborn_and_divine_shield() {
            let s = guard(true);
            let keywords = &s.stats(s.unit_at(P1, 1)).keywords;
            assert!(has_keyword(keywords, KeywordKind::Taunt));
            assert!(has_keyword(keywords, KeywordKind::Reborn));
            assert!(has_keyword(keywords, KeywordKind::DivineShield));
        }
    }
}
