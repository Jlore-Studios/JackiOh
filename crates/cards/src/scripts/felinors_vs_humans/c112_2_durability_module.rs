//! FvH #112.2 Durability Module (SPEC §8.14 row 112.2): (0) Unit, Token (printed Mythic), 0/1 → 0/2, of the
//! held Felinors vs Humans set. A module FvH #112 Jilliax Deluxe offers and fuses (FV2 #112).
//!   Base:    "Armor 1"
//!   Radiant: "Armor 2"
//! Engine: Armor (§6.1) is catalog data; its name is not the Durability keyword (FV2's Durability X).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-112-2";

pub fn script() -> CardScripts {
    // Armor 1 (Radiant Armor 2): catalog data only.
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #112.2 Durability Module — SPEC §8.14 row 112.2.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, scenario};

    fn armor_reaches_play(radiant: bool) {
        let s = scenario(json!({ "p1": { "hand": ["core-005"], "field": [{ "def": ID, "radiant": radiant }] } }));
        let view = s.stats(ID);
        assert!(has_keyword(&view.keywords, KeywordKind::Armor), "radiant {radiant}");
        assert!(!has_keyword(&view.keywords, KeywordKind::Durability), "radiant {radiant}");
    }

    #[test]
    fn its_card_declares_no_numbers() {
        crate::register_all();
        assert!(card_def(ID).params.is_none());
    }

    #[test]
    fn s6_1_the_base_face_has_armor_and_no_durability() {
        armor_reaches_play(false);
    }

    #[test]
    fn s6_1_the_radiant_face_has_armor_and_no_durability() {
        armor_reaches_play(true);
    }
}
