//! SPEC §8.14 FvH #15 Unstable Magma-Cell ((1) Unit, Human, Common, 5/5 → 10/10), a card of the held
//! Felinors vs Humans set (R5235000).
//!   Base:    "Brittle 2"
//!   Radiant: "Brittle 3"
//!
//! Engine: Brittle X is the catalog's keyword (§6.1, R385), its count ticking on the field only (R638): it
//! starts as the card enters the field, first drops at its controller's turn start two turns on, then at
//! each one after, and at 0 the card is destroyed. So there is no script to run. Declares no numbers.
//! Built by S01 (#5251).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-015";

pub fn script() -> CardScripts {
    let base = Script::default();
    // Brittle 3 on the Radiant face is catalog data, which the engine's count reads.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// FvH #15 Unstable Magma-Cell — SPEC §8.14 row 15 (R385, R638).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, next_turn_of, scenario};

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    /// p1 plays the Magma-Cell (`radiant` its face) into lane 1: its instance id.
    fn played(radiant: bool) -> (Scenario, String) {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(8), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(8) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        let id = s.unit_at(P1, 1).id;
        (s, id)
    }

    /// R385, R638: it stands through `survives` of its controller's next turn starts and is destroyed at
    /// the one after.
    fn crumbles_after(radiant: bool, survives: usize) {
        let (mut s, id) = played(radiant);
        for start in 1..=survives {
            next_turn_of(&mut s, P1);
            assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(id.clone()), "standing at own start {start}");
        }
        next_turn_of(&mut s, P1);
        s.expect_in_zone(id.as_str(), "graveyard");
    }

    #[test]
    fn declares_no_tuned_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r385_r638_brittle_2_destroys_it_at_its_second_own_turn_start() {
            crumbles_after(false, 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r385_r638_brittle_3_destroys_it_at_its_third_own_turn_start() {
            crumbles_after(true, 2);
        }
    }
}
