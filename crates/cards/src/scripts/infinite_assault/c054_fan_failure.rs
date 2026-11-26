//! SPEC §8.13 IA #54 Fan Failure ((2) Unit, Common, 6/4 → 12/8), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Brittle 2"
//!   Radiant: "Brittle 2, Rush"
//!
//! Keywords only: Brittle 2 and, on the Radiant face, Rush are the catalog's, so there is nothing
//! to run (`Script::default` on both faces, as C+ #6 Wrong-House Attacker's).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-054";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds Rush and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #54 Fan Failure — SPEC §8.13 row 54.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn brittle_of(s: &Scenario, card: &CardInstance) -> Option<i32> {
        s.stats(card).keywords.iter().find_map(|keyword| match keyword {
            Keyword::Brittle { n } => Some(*n),
            _ => None,
        })
    }

    mod base {
        use super::*;

        #[test]
        fn it_has_brittle_2_and_no_rush() {
            let mut s = scenario(json!({ "p1": { "field": [ID] } }));
            let unit = s.unit_at(P1, 1);
            s.expect_stats(&unit, json!({ "attack": 6, "health": 4 }));
            assert_eq!(brittle_of(&s, &unit), Some(2));
            assert!(!has_keyword(&s.stats(&unit).keywords, KeywordKind::Rush));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn it_has_brittle_2_and_rush() {
            let mut s = scenario(json!({ "p1": { "field": [{ "def": ID, "radiant": true }] } }));
            let unit = s.unit_at(P1, 1);
            s.expect_stats(&unit, json!({ "attack": 12, "health": 8 }));
            assert_eq!(brittle_of(&s, &unit), Some(2));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Rush));
        }
    }
}
