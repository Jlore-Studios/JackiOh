//! SPEC §8.14 FvH #12 Mr. Big ((0) Unit, Human, Common, 2/3 → 6/9), a card of the held Felinors vs Humans
//! set (R5235000).
//!   Base:    no text.
//!   Radiant: no text.
//!
//! A card with no text has no hooks, so both Scripts are empty (Core #8 Mr. Vanilla's shape): its Radiant
//! face is its stats alone. Declares no numbers. Built by S01 (#5251).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-012";

pub fn script() -> CardScripts {
    let base = Script::default();
    // No text on either face: the Radiant face is its catalog stats alone.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// FvH #12 Mr. Big — SPEC §8.14 row 12.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    /// p1 plays Mr. Big (`radiant` its face) into lane 1: the Unit there.
    fn played(radiant: bool) -> (Scenario, CardInstance) {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        let unit = s.unit_at(P1, 1);
        (s, unit)
    }

    #[test]
    fn declares_no_tuned_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn played_it_enters_as_a_plain_unit_with_no_keywords() {
            let (mut s, unit) = played(false);
            assert_eq!(unit.def_id, ID);
            s.expect_stats(&unit, json!({ "attack": 2, "health": 3 }));
            assert!(s.stats(&unit).keywords.is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn played_radiant_it_enters_as_a_plain_unit_with_no_keywords() {
            let (mut s, unit) = played(true);
            assert!(unit.radiant);
            s.expect_stats(&unit, json!({ "attack": 6, "health": 9 }));
            assert!(s.stats(&unit).keywords.is_empty());
        }
    }
}
