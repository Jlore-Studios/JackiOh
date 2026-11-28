//! FvH #112.8 Utility Module (SPEC §8.14 row 112.8): (3) Unit, Token (printed Mythic), 3/1 → 6/2, of the
//! held Felinors vs Humans set. A module FvH #112 Jilliax Deluxe offers and fuses (FV2 #112).
//!   Base:    "Pierce, Cleave"
//!   Radiant: "Deft, Pierce, Cleave"
//! Engine: every word is a catalog keyword (§6.1); the Radiant face adds Deft.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-112-8";

pub fn script() -> CardScripts {
    // Keywords only: catalog data.
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #112.8 Utility Module — SPEC §8.14 row 112.8.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, scenario};

    fn keywords(radiant: bool) -> Vec<Keyword> {
        let s = scenario(json!({ "p1": { "hand": ["core-005"], "field": [{ "def": ID, "radiant": radiant }] } }));
        s.stats(ID).keywords
    }

    #[test]
    fn its_card_declares_no_numbers() {
        crate::register_all();
        assert!(card_def(ID).params.is_none());
    }

    #[test]
    fn s6_1_the_base_face_has_pierce_and_cleave() {
        let kinds = keywords(false);
        assert!(has_keyword(&kinds, KeywordKind::Pierce) && has_keyword(&kinds, KeywordKind::Cleave));
        assert!(!has_keyword(&kinds, KeywordKind::Deft));
    }

    #[test]
    fn s6_1_the_radiant_face_adds_deft() {
        let kinds = keywords(true);
        assert!(has_keyword(&kinds, KeywordKind::Pierce) && has_keyword(&kinds, KeywordKind::Cleave));
        assert!(has_keyword(&kinds, KeywordKind::Deft));
    }
}
