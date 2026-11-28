//! FvH #112.13 Indestructible Module (SPEC §8.14 row 112.13; R69): (6) Unit, Token (printed Mythic), 2/2 →
//! 20/20, of the held Felinors vs Humans set. A module FvH #112 Jilliax Deluxe offers and fuses (FV2 #112).
//!   Base and Radiant: "Indestructible"
//! Engine: Indestructible is catalog data (§6.1); the Radiant face keeps the list's 20/20.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-112-13";

pub fn script() -> CardScripts {
    // Indestructible: catalog data only.
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #112.13 Indestructible Module — SPEC §8.14 row 112.13.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, scenario};

    /// A defender whose strike back is far more than either face's Health.
    const CRUSHER: &str = "stand-in (1) 40/40";

    /// It attacks `CRUSHER`, takes a lethal strike back and stays on the field.
    fn survives(radiant: bool) {
        let mut s = scenario(json!({
            "p1": { "hand": ["core-005"], "field": [{ "def": ID, "radiant": radiant }] },
            "p2": { "hand": ["core-005"], "field": [CRUSHER] },
        }));
        let module = s.unit_at(P1, 1);
        let crusher = s.unit_at(P2, 1);
        s.attack(&module, &crusher);
        s.expect_in_zone(module.id.as_str(), "field");
    }

    #[test]
    fn its_card_declares_no_numbers() {
        crate::register_all();
        assert!(card_def(ID).params.is_none());
    }

    #[test]
    fn r69_the_base_face_survives_a_lethal_hit() {
        survives(false);
    }

    #[test]
    fn r69_the_radiant_face_survives_a_lethal_hit() {
        survives(true);
    }
}
