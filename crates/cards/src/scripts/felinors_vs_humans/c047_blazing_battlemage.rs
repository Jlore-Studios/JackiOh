//! SPEC §8.14 FvH #47 Blazing Battlemage ((0) Unit, Human, Common, 1/1 → 3/3), a card of the held Felinors
//! vs Humans set (R5235000).
//!   Base:    "Cry: Make this Radiant."
//!   Radiant: no text.
//!
//! Engine: the base face's Cry (R1) is `set_radiant` of itself (R22), so it turns to its Radiant face on the
//! field, a 3/3 with no text, its damage kept (§5.2), as M #74 Montana Giant does. The Radiant face has no
//! text, so it runs nothing. Declares no numbers. Built by S01 (#5251).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-047";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| vec![set_radiant(json_as(json!({ "target": { "of": "self" } })))])),
            ..Script::default()
        },
        // No text: the Radiant face is its 3/3 alone.
        radiant: Script::default(),
    }
}

// FvH #47 Blazing Battlemage — SPEC §8.14 row 47 (R1, R22).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    /// p1 plays the Battlemage (`radiant` its face) into lane 1: the Unit there.
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
        fn r22_its_cry_makes_it_radiant_a_3_3() {
            let (mut s, unit) = played(false);
            assert!(unit.radiant, "its Cry turned it Radiant");
            assert_eq!(s.count("radiantSet"), 1);
            s.expect_stats(&unit, json!({ "attack": 3, "health": 3, "maxHealth": 3 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn played_radiant_it_is_a_3_3_and_runs_nothing() {
            let (mut s, unit) = played(true);
            assert!(unit.radiant);
            assert_eq!(s.count("radiantSet"), 0, "no Cry to run");
            s.expect_stats(&unit, json!({ "attack": 3, "health": 3, "maxHealth": 3 }));
        }
    }
}
