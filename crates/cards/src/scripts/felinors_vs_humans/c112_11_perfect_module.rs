//! FvH #112.11 Perfect Module (SPEC §8.14 row 112.11): (4) Unit, Token (printed Mythic), 3/2 → 6/4, of the
//! held Felinors vs Humans set. A module FvH #112 Jilliax Deluxe offers and fuses (FV2 #112).
//!   Base:    "Rush, Taunt, Lifesteal, Divine Shield"
//!   Radiant: "Rush, Taunt, Lifesteal, Divine Shield, Reborn"
//! Engine: every word is a catalog keyword (§6.1); the Radiant face adds Reborn.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-112-11";

pub fn script() -> CardScripts {
    // Keywords only: catalog data.
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #112.11 Perfect Module — SPEC §8.14 row 112.11.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, scenario};

    const PRINTED: [KeywordKind; 4] = [
        KeywordKind::Rush,
        KeywordKind::Taunt,
        KeywordKind::Lifesteal,
        KeywordKind::DivineShield,
    ];

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
    fn s6_1_the_base_face_has_its_four_keywords() {
        let kinds = keywords(false);
        for kind in PRINTED {
            assert!(has_keyword(&kinds, kind), "{kind:?}");
        }
        assert!(!has_keyword(&kinds, KeywordKind::Reborn));
    }

    #[test]
    fn s6_1_the_radiant_face_adds_reborn() {
        let kinds = keywords(true);
        for kind in PRINTED {
            assert!(has_keyword(&kinds, kind), "{kind:?}");
        }
        assert!(has_keyword(&kinds, KeywordKind::Reborn));
    }
}
