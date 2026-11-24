//! IA #27 Decoy Projector (SPEC §8.13 row 27), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Taunt. Cry: Set a Figment."
//!   Radiant: "Taunt, Divine Shield. Cry: Set a Figment."
//!
//! Taunt and the Radiant face's Divine Shield are catalog data. The Cry sets one face-down
//! Figment on both faces (`c025_1_figment::set_figments`); with no open backrow zone, nothing
//! happens (R4586000).
//!
//! Engine: `summon` of a Trap, `setCostOverrideRandom` (R4586000). Built by C2 (#4586).

use jackioh_engine::prelude::*;

use crate::scripts::infinite_assault::c025_1_figment::set_figments;

pub const ID: &str = "infiniteassault-027";

fn projector() -> Script {
    Script {
        cry: Some(hook(move |_| set_figments(1, false))),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: projector(),
        // The same Cry: a plain Figment on both faces (§7).
        radiant: projector(),
    }
}

// IA #27 Decoy Projector — SPEC §8.13 row 27 (R4586000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FIGMENT: &str = "infiniteassault-025-1";
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn played(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        s.play(ID, json!({}));
        s
    }

    fn figments(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .backrow
            .iter()
            .filter_map(|slot| slot.clone())
            .filter(|card| card.def_id == FIGMENT)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4586000_the_cry_sets_one_plain_figment() {
            let s = played(false, "projector-base");
            let set = figments(&s);
            assert_eq!(set.len(), 1);
            assert!(is_face_down(s.state(), &set[0]));
            assert!(!set[0].radiant);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586000_the_cry_sets_one_plain_figment_beside_the_shield() {
            let s = played(true, "projector-radiant");
            let set = figments(&s);
            assert_eq!(set.len(), 1);
            assert!(!set[0].radiant, "plain on both faces");
        }
    }
}
