//! FvH #112.1 Recursion Module (SPEC §8.14 row 112.1; R5239006): (0) Unit, Token (printed Mythic), 1/1 →
//! 2/2, of the held Felinors vs Humans set. A module FvH #112 Jilliax Deluxe offers and fuses (FV2 #112).
//!   Base:    "Flashback / Played from the graveyard by Flashback, this runs no Cry."
//!   Radiant: "Flashback"
//! Engine: Flashback is catalog data. Base: `StaticFlags.flashback_without_cry` (R5239006): a play that
//! spends Flashback runs no Cry, so a fused Jilliax carrying it keeps its modules and discovers none. Radiant:
//! plain Flashback, so a replay runs the Cry again (R5262002 reads that replay's X).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-112-1";

pub fn script() -> CardScripts {
    // R5239006: "Played from the graveyard by Flashback, this runs no Cry."
    let base = Script {
        static_flags: Some(StaticFlags {
            flashback_without_cry: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    // Plain Flashback, catalog data.
    let radiant = Script::default();
    CardScripts { base, radiant }
}

// FvH #112.1 Recursion Module — SPEC §8.14 row 112.1, R5239006.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, scenario};

    fn on_field(radiant: bool) -> Scenario {
        scenario(json!({ "p1": { "hand": ["core-005"], "field": [{ "def": ID, "radiant": radiant }] } }))
    }

    fn quiet(s: &Scenario) -> Option<bool> {
        jackioh_engine::scripts::flags_of(s.state(), &s.unit_at(P1, 1)).flashback_without_cry
    }

    #[test]
    fn its_card_declares_no_numbers() {
        crate::register_all();
        assert!(card_def(ID).params.is_none());
    }

    #[test]
    fn r5239006_the_base_face_flashes_back_without_its_cry() {
        let s = on_field(false);
        assert!(has_keyword(&s.stats(ID).keywords, KeywordKind::Flashback));
        assert_eq!(quiet(&s), Some(true));
    }

    #[test]
    fn r5239006_the_radiant_face_flashes_back_with_its_cry() {
        let s = on_field(true);
        assert!(has_keyword(&s.stats(ID).keywords, KeywordKind::Flashback));
        assert_eq!(quiet(&s), None);
    }
}
