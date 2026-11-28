//! FvH #112.7 Heavy Module (SPEC §8.14 row 112.7): (3) Unit, Token (printed Mythic), 5/5 → 15/15, of the
//! held Felinors vs Humans set. A module FvH #112 Jilliax Deluxe offers and fuses (FV2 #112).
//!   Base and Radiant: no text.
//! Engine: stats only, catalog data; the Radiant face keeps the list's 15/15.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-112-7";

pub fn script() -> CardScripts {
    // Stats only: catalog data.
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #112.7 Heavy Module — SPEC §8.14 row 112.7.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, scenario};

    /// It hits the enemy hero for its printed Attack and does nothing else.
    fn hits_for(radiant: bool, attack: i32) {
        let mut s = scenario(json!({
            "p1": { "hand": ["core-005"], "field": [{ "def": ID, "radiant": radiant }] },
            "p2": { "hand": ["core-005"] },
        }));
        let heavy = s.unit_at(P1, 1);
        s.attack(&heavy, "hero");
        s.expect_health(P2, HERO_HEALTH - attack);
        assert_eq!(s.count("summoned"), 0);
    }

    #[test]
    fn its_card_declares_no_numbers() {
        crate::register_all();
        assert!(card_def(ID).params.is_none());
    }

    #[test]
    fn the_base_face_plays_as_a_vanilla_5_5() {
        hits_for(false, 5);
    }

    #[test]
    fn the_radiant_face_plays_as_a_vanilla_15_15() {
        hits_for(true, 15);
    }
}
