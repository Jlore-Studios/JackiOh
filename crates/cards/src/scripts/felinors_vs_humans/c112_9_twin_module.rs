//! FvH #112.9 Twin Module (SPEC §8.14 row 112.9; R5262001): (4) Unit, Token (printed Mythic), 3/3 → 6/6, of
//! the held Felinors vs Humans set. A module FvH #112 Jilliax Deluxe offers and fuses (FV2 #112).
//!   Base:    "When this is fused, summon a copy of the fused card."
//!   Radiant: "When this is fused, summon two copies of the fused card."
//! Engine: no hook of its own (its script is empty). R5262001: "the fused card" is the whole Jilliax after
//! both picks, so #112's own post-step summons the copies once both modules are in (`summon_copy`, no Cry,
//! R1), one (Radiant two), into open unit zones (R64, none with none, R129); the copies summon nothing.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-112-9";

pub fn script() -> CardScripts {
    // R5262001: #112's post-step is the copy; the module itself runs nothing.
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #112.9 Twin Module — SPEC §8.14 row 112.9, R5262001.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, filler, scenario};

    /// Alone on the field it summons nothing, its turns passing.
    fn copies_nothing_alone(radiant: bool) {
        let mut s = scenario(json!({
            "p1": { "hand": ["core-005"], "field": [{ "def": ID, "radiant": radiant }], "library": filler(4) },
            "p2": { "hand": ["core-005"], "library": filler(4) },
        }));
        s.end_turn();
        s.end_turn();
        assert_eq!(s.count("summoned"), 0, "radiant {radiant}");
        assert!(s.unit(P1, 2).is_none() && s.unit(P2, 1).is_none());
    }

    #[test]
    fn its_card_declares_no_numbers() {
        crate::register_all();
        assert!(card_def(ID).params.is_none());
    }

    #[test]
    fn r5262001_the_base_face_alone_copies_nothing() {
        copies_nothing_alone(false);
    }

    #[test]
    fn r5262001_the_radiant_face_alone_copies_nothing() {
        copies_nothing_alone(true);
    }
}
