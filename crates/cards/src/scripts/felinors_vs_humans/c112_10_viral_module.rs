//! FvH #112.10 Viral Module (SPEC §8.14 row 112.10; R23): (4) Unit, Token (printed Mythic), 2/5 → 4/10, of
//! the held Felinors vs Humans set. A module FvH #112 Jilliax Deluxe offers and fuses (FV2 #112).
//!   Base:    "Immune to Spells, Poisonous"
//!   Radiant: "Immutable, Immune to Spells, Poisonous"
//! Engine: every word is a catalog keyword (§6.1); the Radiant face adds Immutable (R23), which lands with
//! #112's one fusion after both picks.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-112-10";

pub fn script() -> CardScripts {
    // Keywords only: catalog data.
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #112.10 Viral Module — SPEC §8.14 row 112.10.
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
    fn s6_1_the_base_face_is_immune_to_spells_and_poisonous() {
        let kinds = keywords(false);
        assert!(has_keyword(&kinds, KeywordKind::ImmuneToSpells) && has_keyword(&kinds, KeywordKind::Poisonous));
        assert!(!has_keyword(&kinds, KeywordKind::Immutable));
    }

    #[test]
    fn r23_the_radiant_face_adds_immutable() {
        let kinds = keywords(true);
        assert!(has_keyword(&kinds, KeywordKind::ImmuneToSpells) && has_keyword(&kinds, KeywordKind::Poisonous));
        assert!(has_keyword(&kinds, KeywordKind::Immutable));
    }
}
