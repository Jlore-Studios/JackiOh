//! SPEC §8.13 IA #151 Patch Deployment ((2) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Give your Units +{attack}/+{health}." (1/2)
//!   Radiant: "Give your Units +{attack}/+{health}." (2/4)
//!
//! Engine: one `buffAllUnits` over the controller's Units (R4453000: a Unit Immune to Spells is
//! passed by); the Radiant face's numbers are its declared params.

use jackioh_engine::effects::buff_all_units;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-151";

fn deployment() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![buff_all_units(json_as(json!({
                "side": "self",
                "attack": param(&*ctx, "attack"),
                "health": param(&*ctx, "health"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: the Radiant face's +2/+4 are its declared params, read off the running face.
    let base = deployment();
    CardScripts {
        base: base.clone(),
        radiant: base,
    }
}

// IA #151 Patch Deployment — SPEC §8.13 row 151.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/3";
    const FOE: &str = "stand-in (1) 4/4";
    const IMMUNE: &str = "stand-in (1) 2/3 Immune to Spells";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [UNIT, UNIT],
                "library": crate::filler(3),
                "mana": 8,
            },
            "p2": { "field": [FOE], "library": crate::filler(3) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn your_units_gain_1_2_the_enemy_gains_nothing() {
            let mut s = board(false);
            s.play(ID, json!({}));
            s.expect_stats(&s.unit_at(P1, 1), json!({ "attack": 3, "health": 5 }));
            s.expect_stats(&s.unit_at(P1, 2), json!({ "attack": 3, "health": 5 }));
            s.expect_stats(&s.unit_at(P2, 1), json!({ "attack": 4, "health": 4 }));
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn r4453000_a_unit_immune_to_spells_is_passed_by() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": ID }], "field": [IMMUNE], "library": crate::filler(3), "mana": 8 },
                "p2": { "library": crate::filler(3) },
            }));
            s.play(ID, json!({}));
            s.expect_stats(&s.unit_at(P1, 1), json!({ "attack": 2, "health": 3 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn your_units_gain_2_4() {
            let mut s = board(true);
            s.play(ID, json!({}));
            s.expect_stats(&s.unit_at(P1, 1), json!({ "attack": 4, "health": 7 }));
            s.expect_stats(&s.unit_at(P1, 2), json!({ "attack": 4, "health": 7 }));
        }
    }
}
