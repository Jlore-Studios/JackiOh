//! SPEC §8.13 IA #57 Speed Demon ((3) Unit, Common, 6/5 → 12/10), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Charge"
//!   Radiant: "Charge, Windfury"
//!
//! Keywords only: Charge and, on the Radiant face, Windfury are the catalog's, so there is nothing
//! to run (`Script::default` on both faces, as C+ #6 Wrong-House Attacker's).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-057";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds Windfury and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #57 Speed Demon — SPEC §8.13 row 57.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    mod base {
        use super::*;

        #[test]
        fn it_has_charge_and_no_windfury() {
            let mut s = scenario(json!({ "p1": { "field": [ID] } }));
            let unit = s.unit_at(P1, 1);
            s.expect_stats(&unit, json!({ "attack": 6, "health": 5 }));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Charge));
            assert!(!has_keyword(&s.stats(&unit).keywords, KeywordKind::Windfury));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn it_has_charge_and_windfury() {
            let mut s = scenario(json!({ "p1": { "field": [{ "def": ID, "radiant": true }] } }));
            let unit = s.unit_at(P1, 1);
            s.expect_stats(&unit, json!({ "attack": 12, "health": 10 }));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Charge));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Windfury));
        }
    }
}
