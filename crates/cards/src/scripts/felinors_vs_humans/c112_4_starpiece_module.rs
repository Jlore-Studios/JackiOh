//! FvH #112.4 Starpiece Module (SPEC §8.14 row 112.4; R1852000, R1852001): (1) Unit, Token (printed
//! Mythic), 0/1 → 0/3, of the held Felinors vs Humans set. A module FvH #112 Jilliax Deluxe offers and fuses
//! (FV2 #112).
//!   Base and Radiant: "Starship Piece"
//! Engine: "Starpiece" is Starship Piece, catalog data. A fusion that keeps Jilliax drops a module's printed
//! piece (R1852001), so #112 grants it after the fusion; the fused card then dies into its owner's Dormant
//! Starship (R1852000).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-112-4";

pub fn script() -> CardScripts {
    // Starship Piece: catalog data only; #112's post-step grants it to a fused Jilliax.
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #112.4 Starpiece Module — SPEC §8.14 row 112.4.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, scenario};

    fn is_a_piece(radiant: bool) {
        let s = scenario(json!({ "p1": { "hand": ["core-005"], "field": [{ "def": ID, "radiant": radiant }] } }));
        assert!(has_keyword(&s.stats(ID).keywords, KeywordKind::StarshipPiece), "radiant {radiant}");
    }

    #[test]
    fn its_card_declares_no_numbers() {
        crate::register_all();
        assert!(card_def(ID).params.is_none());
    }

    #[test]
    fn r1852000_the_base_face_is_a_starship_piece() {
        is_a_piece(false);
    }

    #[test]
    fn r1852000_the_radiant_face_is_a_starship_piece() {
        is_a_piece(true);
    }
}
