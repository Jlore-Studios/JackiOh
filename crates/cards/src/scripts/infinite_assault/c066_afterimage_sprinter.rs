//! SPEC §8.13 IA #66 Afterimage Sprinter ((3) Unit, Epic, 8/3 → 16/6), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Charge, Windfury, Brittle 1"
//!   Radiant: "Charge, Windfury, Brittle 1, First Strike"
//!
//! Keywords only: Charge, Windfury, Brittle 1 and, on the Radiant face, First Strike are the
//! catalog's, so there is nothing to run (`Script::default` on both faces, as C+ #6
//! Wrong-House Attacker's).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-066";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds First Strike and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #66 Afterimage Sprinter — SPEC §8.13 row 66.
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
        fn it_has_charge_windfury_and_brittle_1() {
            let mut s = scenario(json!({ "p1": { "field": [ID] } }));
            let unit = s.unit_at(P1, 1);
            s.expect_stats(&unit, json!({ "attack": 8, "health": 3 }));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Charge));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Windfury));
            assert!(!has_keyword(&s.stats(&unit).keywords, KeywordKind::FirstStrike));
            assert_eq!(brittle_of(&s, &unit), Some(1));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn it_has_charge_windfury_brittle_1_and_first_strike() {
            let mut s = scenario(json!({ "p1": { "field": [{ "def": ID, "radiant": true }] } }));
            let unit = s.unit_at(P1, 1);
            s.expect_stats(&unit, json!({ "attack": 16, "health": 6 }));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Charge));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Windfury));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::FirstStrike));
            assert_eq!(brittle_of(&s, &unit), Some(1));
        }
    }
}
